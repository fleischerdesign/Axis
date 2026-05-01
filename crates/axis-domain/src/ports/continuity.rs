use crate::models::continuity::{
    ContinuityStatus, InputEvent, PeerArrangement, PeerConfig, Side,
};
use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ContinuityError {
    #[error("Continuity provider error: {0}")]
    ProviderError(String),
    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    #[error("Not connected")]
    NotConnected,
    #[error("Already connected")]
    AlreadyConnected,
    #[error("PIN rejected")]
    PinRejected,
}

pub type ContinuityStream = StatusStream<ContinuityStatus>;

#[async_trait]
pub trait ContinuityProvider: Send + Sync {
    async fn get_status(&self) -> Result<ContinuityStatus, ContinuityError>;
    async fn subscribe(&self) -> Result<ContinuityStream, ContinuityError>;

    async fn set_enabled(&self, enabled: bool) -> Result<(), ContinuityError>;
    async fn connect_to_peer(&self, peer_id: &str) -> Result<(), ContinuityError>;
    async fn confirm_pin(&self) -> Result<(), ContinuityError>;
    async fn reject_pin(&self) -> Result<(), ContinuityError>;
    async fn disconnect(&self) -> Result<(), ContinuityError>;
    async fn cancel_reconnect(&self) -> Result<(), ContinuityError>;
    async fn unpair(&self, peer_id: &str) -> Result<(), ContinuityError>;

    async fn start_sharing(&self, side: Side, edge_pos: f64) -> Result<(), ContinuityError>;
    async fn stop_sharing(&self, edge_pos: f64) -> Result<(), ContinuityError>;
    async fn send_input(&self, event: InputEvent) -> Result<(), ContinuityError>;
    async fn force_local(&self) -> Result<(), ContinuityError>;

    async fn set_peer_arrangement(&self, arrangement: PeerArrangement) -> Result<(), ContinuityError>;
    async fn update_peer_configs(
        &self,
        configs: HashMap<String, PeerConfig>,
    ) -> Result<(), ContinuityError>;
}

crate::status_provider!(ContinuityProvider, ContinuityStatus, ContinuityError);
