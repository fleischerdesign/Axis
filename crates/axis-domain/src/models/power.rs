use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerStatus {
    pub battery_percentage: f64,
    pub is_charging: bool,
    pub power_profile: String,
    pub has_battery: bool,
}

impl Default for PowerStatus {
    fn default() -> Self {
        Self {
            battery_percentage: 100.0,
            is_charging: false,
            power_profile: String::from("balanced"),
            has_battery: false,
        }
    }
}
