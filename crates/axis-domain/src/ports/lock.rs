use crate::models::lock::LockStatus;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum LockError {
    #[error("Lock provider error: {0}")]
    ProviderError(String),
    #[error("Session lock protocol not supported")]
    NotSupported,
}

pub type LockStream = Pin<Box<dyn Stream<Item = LockStatus> + Send>>;

#[async_trait]
pub trait LockProvider: Send + Sync {
    async fn is_supported(&self) -> Result<bool, LockError>;
    async fn lock(&self) -> Result<(), LockError>;
    async fn unlock(&self) -> Result<(), LockError>;
    async fn authenticate(&self, password: &str) -> Result<bool, LockError>;
    async fn subscribe(&self) -> Result<LockStream, LockError>;
}
