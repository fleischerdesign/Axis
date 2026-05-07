use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClockStatus {
    pub current_time: DateTime<Local>,
}
impl Default for ClockStatus {
    fn default() -> Self {
        Self {
            current_time: chrono::Local::now(),
        }
    }
}
