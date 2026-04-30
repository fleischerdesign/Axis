use crate::models::airplane::AirplaneStatus;
use async_trait::async_trait;
use super::StatusStream;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum AirplaneError {
    #[error("Airplane provider error: {0}")]
    ProviderError(String),
}

pub type AirplaneStream = StatusStream<AirplaneStatus>;

#[async_trait]
pub trait AirplaneProvider: Send + Sync {
    async fn get_status(&self) -> Result<AirplaneStatus, AirplaneError>;
    async fn subscribe(&self) -> Result<AirplaneStream, AirplaneError>;
    async fn set_enabled(&self, enabled: bool) -> Result<(), AirplaneError>;
}

crate::status_provider!(AirplaneProvider, AirplaneStatus, AirplaneError);
