use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirplaneStatus {
    pub enabled: bool,
    pub available: bool,
}
