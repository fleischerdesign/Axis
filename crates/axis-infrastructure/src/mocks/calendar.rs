use async_trait::async_trait;
use axis_domain::models::calendar::CalendarEvent;
use axis_domain::ports::calendar::{CalendarError, CalendarProvider};
use chrono::NaiveDate;

pub struct MockCalendarProvider;

#[async_trait]
impl CalendarProvider for MockCalendarProvider {
    async fn get_events(&self, _s: &str, _e: &str) -> Result<Vec<CalendarEvent>, CalendarError> {
        Ok(vec![CalendarEvent {
            id: "1".to_string(),
            summary: "Architecture Review".to_string(),
            start: NaiveDate::from_ymd_opt(2026, 4, 17)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 4, 17)
                .unwrap()
                .and_hms_opt(11, 0, 0)
                .unwrap(),
            all_day: false,
            color_id: Some("9".to_string()),
        }])
    }
}
