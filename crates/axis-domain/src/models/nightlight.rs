use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NightlightStatus {
    pub enabled: bool,
    pub available: bool,
    pub temp_day: u32,
    pub temp_night: u32,
    pub schedule_enabled: bool,
    pub sunrise: String,
    pub sunset: String,
}

impl Default for NightlightStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            available: false,
            temp_day: 6500,
            temp_night: 4500,
            schedule_enabled: false,
            sunrise: String::new(),
            sunset: String::new(),
        }
    }
}
