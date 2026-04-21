use crate::models::bluetooth::BluetoothStatus;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum BluetoothError {
    #[error("Bluetooth provider error: {0}")]
    ProviderError(String),
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

pub type BluetoothStatusStream = Pin<Box<dyn Stream<Item = BluetoothStatus> + Send>>;

#[async_trait]
pub trait BluetoothProvider: Send + Sync {
    async fn get_status(&self) -> Result<BluetoothStatus, BluetoothError>;
    async fn subscribe(&self) -> Result<BluetoothStatusStream, BluetoothError>;
    async fn connect(&self, id: &str) -> Result<(), BluetoothError>;
    async fn disconnect(&self, id: &str) -> Result<(), BluetoothError>;
    async fn set_powered(&self, powered: bool) -> Result<(), BluetoothError>;
    async fn start_scan(&self) -> Result<(), BluetoothError>;
    async fn stop_scan(&self) -> Result<(), BluetoothError>;
}
