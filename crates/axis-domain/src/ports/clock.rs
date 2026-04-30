use crate::models::clock::TimeStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ClockError {
    #[error("Clock provider error: {0}")]
    ProviderError(String),
}

pub type ClockStream = StatusStream<TimeStatus>;

#[async_trait]
pub trait ClockProvider: Send + Sync {
    async fn get_status(&self) -> Result<TimeStatus, ClockError>;
    async fn subscribe(&self) -> Result<ClockStream, ClockError>;
}

crate::status_provider!(ClockProvider, TimeStatus, ClockError);
