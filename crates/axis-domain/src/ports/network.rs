use super::StatusStream;
use crate::models::network::NetworkStatus;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum NetworkError {
    #[error("Network provider error: {0}")]
    ProviderError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Access point not found: {0}")]
    AccessPointNotFound(String),
    #[error("Authentication failed")]
    AuthFailed,
}

pub type NetworkStream = StatusStream<NetworkStatus>;

/// Provides WiFi network state (scan results, connection status) and allows connect/disconnect/scan.
#[async_trait]
pub trait NetworkProvider: Send + Sync {
    async fn get_status(&self) -> Result<NetworkStatus, NetworkError>;
    async fn subscribe(&self) -> Result<NetworkStream, NetworkError>;
    async fn set_wifi_enabled(&self, enabled: bool) -> Result<(), NetworkError>;
    async fn scan_wifi(&self) -> Result<(), NetworkError>;
    async fn connect_to_ap(&self, id: &str, password: Option<&str>) -> Result<(), NetworkError>;
    async fn disconnect_wifi(&self) -> Result<(), NetworkError>;
}

crate::status_provider!(NetworkProvider, NetworkStatus, NetworkError);
