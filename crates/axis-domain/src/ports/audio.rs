use crate::models::audio::AudioStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum AudioError {
    #[error("Audio provider error: {0}")]
    ProviderError(String),
}

pub type AudioStream = StatusStream<AudioStatus>;

#[async_trait]
pub trait AudioProvider: Send + Sync {
    async fn get_status(&self) -> Result<AudioStatus, AudioError>;
    async fn set_volume(&self, volume: f64) -> Result<(), AudioError>;
    async fn set_muted(&self, muted: bool) -> Result<(), AudioError>;
    async fn subscribe(&self) -> Result<AudioStream, AudioError>;
    async fn set_default_sink(&self, id: u32) -> Result<(), AudioError>;
    async fn set_default_source(&self, id: u32) -> Result<(), AudioError>;
    async fn set_sink_input_volume(&self, id: u32, volume: f64) -> Result<(), AudioError>;
}

crate::status_provider!(AudioProvider, AudioStatus, AudioError);
