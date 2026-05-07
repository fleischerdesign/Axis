use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum PowerProfile {
    #[default]
    Balanced,
    Performance,
    PowerSaver,
    Custom(Box<str>),
}

impl PowerProfile {
    pub fn custom(name: &str) -> Option<Self> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self::Custom(trimmed.into()))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerStatus {
    pub battery_percentage: f64,
    pub is_charging: bool,
    pub power_profile: PowerProfile,
    pub has_battery: bool,
}

impl Default for PowerStatus {
    fn default() -> Self {
        Self {
            battery_percentage: 100.0,
            is_charging: false,
            power_profile: PowerProfile::Balanced,
            has_battery: false,
        }
    }
}
