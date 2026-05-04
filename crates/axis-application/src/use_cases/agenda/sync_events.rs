use axis_domain::models::calendar::CalendarEvent;
use axis_domain::ports::agenda::{AgendaProvider, AgendaError};
use std::sync::Arc;
use chrono::{Utc, Duration};
use log::info;

pub struct SyncEventsUseCase {
    provider: Arc<dyn AgendaProvider>,
}

impl SyncEventsUseCase {
    pub fn new(provider: Arc<dyn AgendaProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<Vec<CalendarEvent>, AgendaError> {
        let now = Utc::now();
        let start = (now - Duration::days(30)).to_rfc3339();
        let end = (now + Duration::days(60)).to_rfc3339();

        info!("[use-case] Syncing calendar events");
        self.provider.fetch_events(&start, &end).await
    }
}
