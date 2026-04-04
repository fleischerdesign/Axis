pub mod provider;
pub mod google;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub use provider::{AuthStatus, CalendarEvent, CalendarProvider, DateRange};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarData {
    pub events: Vec<CalendarEvent>,
    pub selected_range: DateRange,
    pub is_authenticated: bool,
    pub loading: bool,
}

impl Default for CalendarData {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            selected_range: DateRange::Today,
            is_authenticated: false,
            loading: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CalendarCmd {
    SetRange(DateRange),
    Refresh,
}

pub struct CalendarRegistry {
    provider: Box<dyn CalendarProvider>,
    cached_events: Vec<CalendarEvent>,
    month_events: Vec<CalendarEvent>,
    selected_range: DateRange,
}

impl CalendarRegistry {
    pub fn new() -> Self {
        let provider = Box::new(google::GoogleCalendarProvider::new());

        Self {
            provider,
            cached_events: Vec::new(),
            month_events: Vec::new(),
            selected_range: DateRange::Today,
        }
    }

    pub fn provider_mut(&mut self) -> &mut dyn CalendarProvider {
        &mut *self.provider
    }

    pub fn is_authenticated(&self) -> bool {
        self.provider.is_authenticated()
    }

    pub fn auth_status(&mut self) -> provider::AuthStatus {
        self.provider.auth_status()
    }

    pub fn authenticate(&mut self) -> Result<provider::AuthStatus, String> {
        self.provider.authenticate()
    }

    pub fn refresh_events(&mut self) -> Result<Vec<CalendarEvent>, String> {
        let (start, end) = get_date_range(self.selected_range);
        let events = self.provider.events(&start, &end)?;
        self.cached_events = events.clone();
        Ok(events)
    }

    pub fn set_range(&mut self, range: DateRange) {
        self.selected_range = range;
    }

    pub fn cached_events(&self) -> &[CalendarEvent] {
        &self.cached_events
    }

    pub fn selected_range(&self) -> DateRange {
        self.selected_range
    }

    pub fn refresh_month_events(&mut self, year: i32, month: u32) -> Result<Vec<CalendarEvent>, String> {
        let (start, end) = get_month_range(year, month);
        let events = self.provider.events(&start, &end)?;
        self.month_events = events.clone();
        Ok(events)
    }

    pub fn month_events(&self) -> &[CalendarEvent] {
        &self.month_events
    }
}

fn get_date_range(range: DateRange) -> (String, String) {
    let now = chrono::Local::now();
    match range {
        DateRange::Today => {
            let start = now.format("%Y-%m-%dT00:00:00Z").to_string();
            let end = now.format("%Y-%m-%dT23:59:59Z").to_string();
            (start, end)
        }
        DateRange::Week => {
            let start = now.format("%Y-%m-%dT00:00:00Z").to_string();
            let end = (now + chrono::Duration::days(7)).format("%Y-%m-%dT23:59:59Z").to_string();
            (start, end)
        }
    }
}

fn get_month_range(year: i32, month: u32) -> (String, String) {
    let start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .unwrap_or_default()
        .and_hms_opt(0, 0, 0).unwrap();
    use chrono::Datelike;
    let last_day = chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(30);
    let end = chrono::NaiveDate::from_ymd_opt(year, month, last_day)
        .unwrap_or_default()
        .and_hms_opt(23, 59, 59).unwrap();
    (
        format!("{}Z", start.format("%Y-%m-%dT%H:%M:%S")),
        format!("{}Z", end.format("%Y-%m-%dT%H:%M:%S")),
    )
}