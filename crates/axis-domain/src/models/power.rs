use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerStatus {
    pub battery_percentage: f64,
    pub is_charging: bool,
    pub power_profile: String,
    pub has_battery: bool,
}
