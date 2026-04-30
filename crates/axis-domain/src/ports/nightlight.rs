use crate::models::nightlight::NightlightStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum NightlightError {
    #[error("Nightlight provider error: {0}")]
    ProviderError(String),
}

pub type NightlightStream = StatusStream<NightlightStatus>;

#[async_trait]
pub trait NightlightProvider: Send + Sync {
    async fn get_status(&self) -> Result<NightlightStatus, NightlightError>;
    async fn subscribe(&self) -> Result<NightlightStream, NightlightError>;
    async fn set_enabled(&self, enabled: bool) -> Result<(), NightlightError>;
    async fn set_temp_day(&self, temp: u32) -> Result<(), NightlightError>;
    async fn set_temp_night(&self, temp: u32) -> Result<(), NightlightError>;
    async fn set_schedule(&self, sunrise: &str, sunset: &str) -> Result<(), NightlightError>;
}

crate::status_provider!(NightlightProvider, NightlightStatus, NightlightError);
