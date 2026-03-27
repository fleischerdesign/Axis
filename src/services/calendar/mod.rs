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
    selected_range: DateRange,
}

impl CalendarRegistry {
    pub fn new() -> Self {
        let provider = Box::new(google::GoogleCalendarProvider::new());

        Self {
            provider,
            cached_events: Vec::new(),
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