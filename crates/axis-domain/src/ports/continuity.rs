use crate::models::continuity::{Peer, ContinuityMessage};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ContinuityError {
    #[error("Continuity provider error: {0}")]
    ProviderError(String),
    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

#[async_trait]
pub trait PeerDiscovery: Send + Sync {
    async fn start_browsing(&self) -> Result<(), ContinuityError>;
    async fn stop_browsing(&self) -> Result<(), ContinuityError>;
    async fn get_discovered_peers(&self) -> Result<Vec<Peer>, ContinuityError>;
}

#[async_trait]
pub trait PeerConnection: Send + Sync {
    async fn connect(&self, peer: &Peer) -> Result<(), ContinuityError>;
    async fn disconnect(&self) -> Result<(), ContinuityError>;
    async fn send_message(&self, msg: ContinuityMessage) -> Result<(), ContinuityError>;
}
