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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_profile_custom_empty_string() {
        assert_eq!(PowerProfile::custom(""), None);
        assert_eq!(PowerProfile::custom("  "), None);
    }

    #[test]
    fn power_profile_custom_non_empty() {
        let p = PowerProfile::custom("gaming").unwrap();
        assert_eq!(p, PowerProfile::Custom("gaming".into()));
    }

    #[test]
    fn power_status_default_has_no_battery() {
        let s = PowerStatus::default();
        assert!(!s.has_battery);
        assert_eq!(s.battery_percentage, 100.0);
    }

    #[test]
    fn power_profile_serde_roundtrip() {
        let cases = vec![
            PowerProfile::Balanced,
            PowerProfile::Performance,
            PowerProfile::PowerSaver,
            PowerProfile::Custom("gaming".into()),
        ];
        for profile in cases {
            let json = serde_json::to_string(&profile).unwrap();
            let back: PowerProfile = serde_json::from_str(&json).unwrap();
            assert_eq!(profile, back);
        }
    }

    #[test]
    fn power_status_serde_roundtrip() {
        let status = PowerStatus {
            battery_percentage: 73.5,
            is_charging: true,
            power_profile: PowerProfile::PowerSaver,
            has_battery: true,
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: PowerStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }
}
