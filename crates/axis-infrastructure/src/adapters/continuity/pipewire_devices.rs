use log::warn;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipeWireAudioDevice {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_sink_monitor: bool,
    pub is_source: bool,
}

async fn resolve_default_monitor() -> Option<PipeWireAudioDevice> {
    let output = Command::new("wpctl")
        .args(["inspect", "@DEFAULT_SINK@"])
        .output()
        .await
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(node_name) = line
            .strip_prefix("* node.name = \"")
            .and_then(|s| s.strip_suffix('"'))
        {
            let monitor_id = format!("{node_name}.monitor");
            return Some(PipeWireAudioDevice {
                id: monitor_id.clone(),
                name: monitor_id,
                description: "System Sound (Default Monitor)".to_string(),
                is_sink_monitor: true,
                is_source: false,
            });
        }
    }
    warn!("[continuity-pw] could not resolve @DEFAULT_SINK@, falling back to @DEFAULT_MONITOR@");
    None
}

pub async fn list_pipewire_audio_devices() -> Vec<PipeWireAudioDevice> {
    let mut devices = Vec::new();

    let default_monitor = resolve_default_monitor().await;
    if let Some(monitor) = default_monitor {
        devices.push(monitor);
    }
    devices.push(PipeWireAudioDevice {
        id: "@DEFAULT_SOURCE@".to_string(),
        name: "@DEFAULT_SOURCE@".to_string(),
        description: "Default Microphone".to_string(),
        is_sink_monitor: false,
        is_source: true,
    });

    if let Ok(output) = Command::new("pw-dump").output().await
        && let Ok(json_str) = String::from_utf8(output.stdout)
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str)
        && let Some(array) = value.as_array()
    {
        for item in array {
            if item.get("type").and_then(|v| v.as_str()) == Some("PipeWire:Interface:Node")
                && let Some(info) = item.get("info")
                && let Some(props) = info.get("props")
            {
                let node_name = props
                    .get("node.name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let node_desc = props
                    .get("node.description")
                    .and_then(|v| v.as_str())
                    .unwrap_or(node_name);
                let media_class = props
                    .get("media.class")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                if node_name.is_empty() {
                    continue;
                }

                if media_class == "Audio/Source" || media_class.contains("Source") {
                    devices.push(PipeWireAudioDevice {
                        id: node_name.to_string(),
                        name: node_name.to_string(),
                        description: format!("{node_desc} (Mic)"),
                        is_sink_monitor: false,
                        is_source: true,
                    });
                } else if media_class == "Audio/Sink" || media_class.contains("Sink") {
                    devices.push(PipeWireAudioDevice {
                        id: format!("{node_name}.monitor"),
                        name: format!("{node_name}.monitor"),
                        description: format!("{node_desc} (Monitor)"),
                        is_sink_monitor: true,
                        is_source: false,
                    });
                }
            }
        }
    }

    devices
}
