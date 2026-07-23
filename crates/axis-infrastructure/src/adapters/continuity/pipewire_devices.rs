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

pub async fn list_pipewire_audio_devices() -> Vec<PipeWireAudioDevice> {
    let mut devices = vec![
        PipeWireAudioDevice {
            id: "@DEFAULT_MONITOR@".to_string(),
            name: "@DEFAULT_MONITOR@".to_string(),
            description: "System Sound (Spotify, Browser, Media)".to_string(),
            is_sink_monitor: true,
            is_source: false,
        },
        PipeWireAudioDevice {
            id: "@DEFAULT_SOURCE@".to_string(),
            name: "@DEFAULT_SOURCE@".to_string(),
            description: "Default Microphone".to_string(),
            is_sink_monitor: false,
            is_source: true,
        },
    ];

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
