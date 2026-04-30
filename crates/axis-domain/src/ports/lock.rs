use crate::models::lock::LockStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LockError {
    #[error("Lock provider error: {0}")]
    ProviderError(String),
    #[error("Session lock protocol not supported")]
    NotSupported,
}

pub type LockStream = StatusStream<LockStatus>;

#[async_trait]
pub trait LockProvider: Send + Sync {
    async fn get_status(&self) -> Result<LockStatus, LockError>;
    async fn is_supported(&self) -> Result<bool, LockError>;
    async fn lock(&self) -> Result<(), LockError>;
    async fn unlock(&self) -> Result<(), LockError>;
    async fn authenticate(&self, password: &str) -> Result<bool, LockError>;
    async fn subscribe(&self) -> Result<LockStream, LockError>;
}

crate::status_provider!(LockProvider, LockStatus, LockError);
