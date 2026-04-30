use crate::models::brightness::BrightnessStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum BrightnessError {
    #[error("Brightness provider error: {0}")]
    ProviderError(String),
}

pub type BrightnessStream = StatusStream<BrightnessStatus>;

#[async_trait]
pub trait BrightnessProvider: Send + Sync {
    async fn get_status(&self) -> Result<BrightnessStatus, BrightnessError>;
    async fn subscribe(&self) -> Result<BrightnessStream, BrightnessError>;
    async fn set_brightness(&self, percentage: f64) -> Result<(), BrightnessError>;
}

crate::status_provider!(BrightnessProvider, BrightnessStatus, BrightnessError);
