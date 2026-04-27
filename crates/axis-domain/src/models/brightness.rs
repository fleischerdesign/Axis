use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrightnessStatus {
    pub percentage: f64,
    pub has_backlight: bool,
}

impl Default for BrightnessStatus {
    fn default() -> Self {
        Self {
            percentage: 1.0,
            has_backlight: true,
        }
    }
}
