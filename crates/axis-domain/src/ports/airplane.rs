use crate::models::airplane::AirplaneStatus;
use async_trait::async_trait;
use futures_util::Stream;
use std::pin::Pin;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AirplaneError {
    #[error("Airplane provider error: {0}")]
    ProviderError(String),
}

pub type AirplaneStream = Pin<Box<dyn Stream<Item = AirplaneStatus> + Send>>;

#[async_trait]
pub trait AirplaneProvider: Send + Sync {
    async fn get_status(&self) -> Result<AirplaneStatus, AirplaneError>;
    async fn subscribe(&self) -> Result<AirplaneStream, AirplaneError>;
    async fn set_enabled(&self, enabled: bool) -> Result<(), AirplaneError>;
}
