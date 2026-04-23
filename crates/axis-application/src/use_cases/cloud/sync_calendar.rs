use axis_domain::models::calendar::CalendarEvent;
use axis_domain::ports::calendar::CalendarProvider;
use std::sync::Arc;
use chrono::{Utc, Duration};

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

        self.provider.get_events(&start, &end).await
            .map_err(|e| e.to_string())
    }
}
