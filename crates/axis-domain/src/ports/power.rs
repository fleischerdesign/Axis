use crate::models::power::PowerStatus;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum PowerError {
    #[error("Power provider error: {0}")]
    ProviderError(String),
}

pub type PowerStream = Pin<Box<dyn Stream<Item = PowerStatus> + Send>>;

#[async_trait]
pub trait PowerProvider: Send + Sync {
    async fn get_status(&self) -> Result<PowerStatus, PowerError>;
    async fn subscribe(&self) -> Result<PowerStream, PowerError>;
    async fn suspend(&self) -> Result<(), PowerError>;
    async fn power_off(&self) -> Result<(), PowerError>;
    async fn reboot(&self) -> Result<(), PowerError>;
}
