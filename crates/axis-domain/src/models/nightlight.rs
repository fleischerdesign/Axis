use serde::{Deserialize, Serialize};
use chrono::NaiveTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NightlightStatus {
    pub enabled: bool,
    pub available: bool,
    pub temp_day: u32,
    pub temp_night: u32,
    pub schedule_enabled: bool,
    pub sunrise: NaiveTime,
    pub sunset: NaiveTime,
}

impl Default for NightlightStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            available: false,
            temp_day: 6500,
            temp_night: 4500,
            schedule_enabled: false,
            sunrise: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
            sunset: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
        }
    }
}
