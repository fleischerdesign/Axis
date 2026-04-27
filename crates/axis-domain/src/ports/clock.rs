use crate::models::clock::TimeStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug)]
pub enum ClockError {
    #[error("Clock provider error: {0}")]
    ProviderError(String),
}

pub type ClockStream = StatusStream<TimeStatus>;

#[async_trait]
pub trait ClockProvider: Send + Sync {
    async fn get_time(&self) -> Result<TimeStatus, ClockError>;
    async fn subscribe(&self) -> Result<ClockStream, ClockError>;
}
