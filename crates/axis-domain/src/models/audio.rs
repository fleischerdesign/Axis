use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AudioStatus {
    pub volume: f64,
    pub is_muted: bool,
    pub sinks: Vec<AudioDevice>,
    pub sources: Vec<AudioDevice>,
    pub sink_inputs: Vec<SinkInput>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct SinkInput {
    pub id: u32,
    pub name: String,
    pub volume: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_status_serde_roundtrip() {
        let status = AudioStatus {
            volume: 0.75,
            is_muted: false,
            sinks: vec![AudioDevice {
                id: 1,
                name: "alsa_output.pci".into(),
                description: "Built-in Audio".into(),
                is_default: true,
            }],
            sources: vec![],
            sink_inputs: vec![SinkInput {
                id: 42,
                name: "Firefox".into(),
                volume: 0.8,
            }],
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: AudioStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }

    #[test]
    fn audio_device_default() {
        let d = AudioDevice::default();
        assert_eq!(d.id, 0);
        assert!(d.name.is_empty());
        assert!(!d.is_default);
    }
}
