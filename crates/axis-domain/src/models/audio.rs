#[derive(Clone, Debug, PartialEq)]
pub struct AudioStatus {
    pub volume: f64,
    pub is_muted: bool,
    pub sinks: Vec<AudioDevice>,
    pub sources: Vec<AudioDevice>,
    pub sink_inputs: Vec<SinkInput>,
}

impl Default for AudioStatus {
    fn default() -> Self {
        Self {
            volume: 0.0,
            is_muted: false,
            sinks: vec![],
            sources: vec![],
            sink_inputs: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SinkInput {
    pub id: u32,
    pub name: String,
    pub volume: f64,
}
