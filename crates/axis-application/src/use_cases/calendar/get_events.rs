use axis_domain::models::calendar::CalendarEvent;
use axis_domain::ports::calendar::{CalendarProvider, CalendarError};
use std::sync::Arc;
use log::debug;

pub struct GetCalendarEventsUseCase {
    provider: Arc<dyn CalendarProvider>,
}

impl GetCalendarEventsUseCase {
    pub fn new(provider: Arc<dyn CalendarProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, start: &str, end: &str) -> Result<Vec<CalendarEvent>, CalendarError> {
        debug!("[use-case] Fetching calendar events between {} and {}", start, end);
        let events = self.provider.get_events(start, end).await?;
        debug!("[use-case] Successfully fetched {} calendar events", events.len());
        Ok(events)
    }
}
