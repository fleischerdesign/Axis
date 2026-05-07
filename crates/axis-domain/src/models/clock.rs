use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local};

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
