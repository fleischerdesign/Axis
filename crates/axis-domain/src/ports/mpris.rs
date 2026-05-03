use crate::models::mpris::MprisStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum MprisError {
    #[error("MPRIS error: {0}")]
    ProviderError(String),
}

pub type MprisStream = StatusStream<MprisStatus>;

#[async_trait]
pub trait MprisProvider: Send + Sync {
    async fn get_status(&self) -> Result<MprisStatus, MprisError>;
    async fn subscribe(&self) -> Result<MprisStream, MprisError>;
    async fn play_pause(&self, player_id: &str) -> Result<(), MprisError>;
    async fn next(&self, player_id: &str) -> Result<(), MprisError>;
    async fn previous(&self, player_id: &str) -> Result<(), MprisError>;
}

crate::status_provider!(MprisProvider, MprisStatus, MprisError);
