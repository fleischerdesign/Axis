use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CalendarEvent {
    pub id: String,
    pub summary: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub all_day: bool,
    pub color_id: Option<String>,
}

impl Default for CalendarEvent {
    fn default() -> Self {
        let epoch = NaiveDateTime::from_timestamp_opt(0, 0).unwrap();
        Self {
            id: String::new(),
            summary: String::new(),
            start: epoch,
            end: epoch,
            all_day: false,
            color_id: None,
        }
    }
}
