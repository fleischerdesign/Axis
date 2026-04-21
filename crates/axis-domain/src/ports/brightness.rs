use crate::models::brightness::BrightnessStatus;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum BrightnessError {
    #[error("Brightness provider error: {0}")]
    ProviderError(String),
}

pub type BrightnessStream = Pin<Box<dyn Stream<Item = BrightnessStatus> + Send>>;

#[async_trait]
pub trait BrightnessProvider: Send + Sync {
    async fn get_status(&self) -> Result<BrightnessStatus, BrightnessError>;
    async fn subscribe(&self) -> Result<BrightnessStream, BrightnessError>;
    async fn set_brightness(&self, percentage: f64) -> Result<(), BrightnessError>;
}
