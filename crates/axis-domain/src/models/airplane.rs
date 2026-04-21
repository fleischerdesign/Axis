use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirplaneStatus {
    pub enabled: bool,
    pub available: bool,
}

impl Default for AirplaneStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            available: false,
        }
    }
}
