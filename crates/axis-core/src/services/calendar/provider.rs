use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub summary: String,
    pub start: String,
    pub end: String,
    pub all_day: bool,
    pub location: Option<String>,
    pub color_id: Option<String>,
}

impl CalendarEvent {
    pub fn format_time_range(&self) -> String {
        if self.all_day {
            "Ganztägig".to_string()
        } else if let (Some(start), Some(end)) = (self.start.split('T').nth(1), self.end.split('T').nth(1)) {
            let start_time = start.split('+').next().unwrap_or(start);
            let end_time = end.split('+').next().unwrap_or(end);
            format!("{} - {}", &start_time[..5], &end_time[..5])
        } else {
            "Ganztägig".to_string()
        }
    }
}

#[derive(Clone, Debug)]
pub enum AuthStatus {
    Authenticated,
    NeedsAuth { url: String },
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateRange {
    Today,
    Week,
}

pub trait CalendarProvider: Send + Sync {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;

    fn is_async(&self) -> bool {
        true
    }

    fn auth_status(&mut self) -> AuthStatus {
        AuthStatus::Authenticated
    }

    fn authenticate(&mut self) -> Result<AuthStatus, String> {
        Ok(AuthStatus::Authenticated)
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    fn events(&mut self, start: &str, end: &str) -> Result<Vec<CalendarEvent>, String>;
}