use async_trait::async_trait;
use thiserror::Error;
use crate::models::cloud::CloudAccount;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum AuthError {
    #[error("Authentication provider error: {0}")]
    ProviderError(String),
    #[error("Network error: {0}")]
    NetworkFailed(String),
    #[error("User cancelled")]
    Cancelled,
}

#[async_trait]
pub trait CloudAuthProvider: Send + Sync {
    async fn authenticate(&self, scopes: &[String]) -> Result<CloudAccount, AuthError>;
    async fn get_token(&self, scopes: &[String]) -> Result<String, AuthError>;
    async fn is_authenticated(&self) -> Result<bool, AuthError>;
}
