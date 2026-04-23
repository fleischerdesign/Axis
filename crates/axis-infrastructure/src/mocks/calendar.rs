use axis_domain::models::calendar::CalendarEvent;
use axis_domain::ports::calendar::{CalendarProvider, CalendarError};
use async_trait::async_trait;

pub struct MockCalendarProvider;

#[async_trait]
impl CalendarProvider for MockCalendarProvider {
    async fn get_events(&self, _s: &str, _e: &str) -> Result<Vec<CalendarEvent>, CalendarError> {
        Ok(vec![
            CalendarEvent {
                id: "1".to_string(),
                summary: "Architecture Review".to_string(),
                start: "2026-04-17T10:00:00".to_string(),
                end: "2026-04-17T11:00:00".to_string(),
                all_day: false,
                color_id: Some("9".to_string()),
            }
        ])
    }
}
