use axis_domain::models::calendar::CalendarEvent;
use axis_domain::ports::calendar::CalendarProvider;
use std::sync::Arc;
use chrono::{Utc, Duration};
use log::{info, error};

pub struct SyncCalendarUseCase {
    provider: Arc<dyn CalendarProvider>,
}

impl SyncCalendarUseCase {
    pub fn new(provider: Arc<dyn CalendarProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<Vec<CalendarEvent>, String> {
        let now = Utc::now();
        let start = (now - Duration::days(30)).to_rfc3339();
        let end = (now + Duration::days(60)).to_rfc3339();

        info!("[use-case] Syncing calendar events");

        match self.provider.get_events(&start, &end).await {
            Ok(events) => {
                info!("[use-case] Calendar sync complete ({} events)", events.len());
                Ok(events)
            },
            Err(e) => {
                let err_msg = e.to_string();
                error!("[use-case] Calendar sync failed: {}", err_msg);
                Err(err_msg)
            }
        }
    }
}
