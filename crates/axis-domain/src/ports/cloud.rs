use crate::models::cloud::{CloudStatus, CloudAccount};
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum CloudError {
    #[error("Cloud provider error: {0}")]
    ProviderError(String),
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
}

pub type CloudStream = StatusStream<CloudStatus>;

#[async_trait]
pub trait CloudProvider: Send + Sync {
    async fn get_status(&self) -> Result<CloudStatus, CloudError>;
    async fn subscribe(&self) -> Result<CloudStream, CloudError>;
    async fn add_account(&self, account: CloudAccount) -> Result<(), CloudError>;
    async fn remove_account(&self, account_id: &str) -> Result<(), CloudError>;
}

crate::status_provider!(CloudProvider, CloudStatus, CloudError);
