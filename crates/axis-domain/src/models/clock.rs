use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeStatus {
    pub current_time: DateTime<Local>,
}
