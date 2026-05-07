use super::StatusStream;
use crate::models::clock::ClockStatus;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ClockError {
    #[error("Clock provider error: {0}")]
    ProviderError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type ClockStream = StatusStream<ClockStatus>;

#[async_trait]
pub trait ClockProvider: Send + Sync {
    async fn get_status(&self) -> Result<ClockStatus, ClockError>;
    async fn subscribe(&self) -> Result<ClockStream, ClockError>;
}

crate::status_provider!(ClockProvider, ClockStatus, ClockError);
