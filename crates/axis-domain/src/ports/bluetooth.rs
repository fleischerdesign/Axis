use crate::models::bluetooth::BluetoothStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum BluetoothError {
    #[error("Bluetooth provider error: {0}")]
    ProviderError(String),
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

pub type BluetoothStream = StatusStream<BluetoothStatus>;

#[async_trait]
pub trait BluetoothProvider: Send + Sync {
    async fn get_status(&self) -> Result<BluetoothStatus, BluetoothError>;
    async fn subscribe(&self) -> Result<BluetoothStream, BluetoothError>;
    async fn connect(&self, id: &str) -> Result<(), BluetoothError>;
    async fn disconnect(&self, id: &str) -> Result<(), BluetoothError>;
    async fn set_powered(&self, powered: bool) -> Result<(), BluetoothError>;
    async fn start_scan(&self) -> Result<(), BluetoothError>;
    async fn stop_scan(&self) -> Result<(), BluetoothError>;
    async fn pair_accept(&self) -> Result<(), BluetoothError>;
    async fn pair_reject(&self) -> Result<(), BluetoothError>;
}

crate::status_provider!(BluetoothProvider, BluetoothStatus, BluetoothError);
