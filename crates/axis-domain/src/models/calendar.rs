use chrono::{DateTime, NaiveDateTime};
use serde::{Deserialize, Serialize};

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
        let epoch = DateTime::from_timestamp(0, 0).unwrap().naive_utc();
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
