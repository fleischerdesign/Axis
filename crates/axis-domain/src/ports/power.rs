use crate::models::power::PowerStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum PowerError {
    #[error("Power provider error: {0}")]
    ProviderError(String),
}

pub type PowerStream = StatusStream<PowerStatus>;

#[async_trait]
pub trait PowerProvider: Send + Sync {
    async fn get_status(&self) -> Result<PowerStatus, PowerError>;
    async fn subscribe(&self) -> Result<PowerStream, PowerError>;
    async fn suspend(&self) -> Result<(), PowerError>;
    async fn power_off(&self) -> Result<(), PowerError>;
    async fn reboot(&self) -> Result<(), PowerError>;
}

crate::status_provider!(PowerProvider, PowerStatus, PowerError);
