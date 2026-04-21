use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrightnessStatus {
    pub percentage: f64,
    pub has_backlight: bool,
}
