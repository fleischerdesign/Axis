use crate::models::clock::TimeStatus;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum ClockError {
    #[error("Clock provider error: {0}")]
    ProviderError(String),
}

pub type ClockStream = Pin<Box<dyn Stream<Item = TimeStatus> + Send>>;

#[async_trait]
pub trait ClockProvider: Send + Sync {
    async fn get_time(&self) -> Result<TimeStatus, ClockError>;
    async fn subscribe(&self) -> Result<ClockStream, ClockError>;
}
