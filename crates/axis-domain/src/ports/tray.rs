use crate::models::tray::TrayStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug)]
pub enum TrayError {
    #[error("Tray provider error: {0}")]
    ProviderError(String),
}

pub type TrayStream = StatusStream<TrayStatus>;

#[async_trait]
pub trait TrayProvider: Send + Sync {
    async fn get_status(&self) -> Result<TrayStatus, TrayError>;
    async fn subscribe(&self) -> Result<TrayStream, TrayError>;
    async fn activate(&self, bus_name: &str, x: i32, y: i32) -> Result<(), TrayError>;
    async fn context_menu(&self, bus_name: &str, x: i32, y: i32) -> Result<(), TrayError>;
    async fn secondary_activate(&self, bus_name: &str, x: i32, y: i32) -> Result<(), TrayError>;
    async fn scroll(&self, bus_name: &str, delta: i32, orientation: &str) -> Result<(), TrayError>;
}
