use crate::models::cloud::{CloudStatus, CloudAccount};
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum CloudError {
    #[error("Cloud provider error: {0}")]
    ProviderError(String),
    #[error("Authentication failed: {0}")]
    AuthError(String),
}

pub type CloudStream = Pin<Box<dyn Stream<Item = CloudStatus> + Send>>;

#[async_trait]
pub trait CloudProvider: Send + Sync {
    async fn get_status(&self) -> Result<CloudStatus, CloudError>;
    async fn subscribe(&self) -> Result<CloudStream, CloudError>;
    async fn add_account(&self, account: CloudAccount) -> Result<(), CloudError>;
    async fn remove_account(&self, account_id: &str) -> Result<(), CloudError>;
}
