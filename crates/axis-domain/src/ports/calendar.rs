use crate::models::calendar::CalendarEvent;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CalendarError {
    #[error("Calendar provider error: {0}")]
    ProviderError(String),
}

#[async_trait]
pub trait CalendarProvider: Send + Sync {
    async fn get_events(&self, start: &str, end: &str) -> Result<Vec<CalendarEvent>, CalendarError>;
}
