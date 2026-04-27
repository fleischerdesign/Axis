use async_trait::async_trait;
use thiserror::Error;
use crate::models::cloud::CloudAccount;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Authentication failed: {0}")]
    Failed(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("User cancelled")]
    Cancelled,
}

#[async_trait]
pub trait CloudAuthProvider: Send + Sync {
    async fn authenticate(&self, scopes: &[String]) -> Result<CloudAccount, AuthError>;
    async fn get_token(&self, scopes: &[String]) -> Result<String, AuthError>;
    async fn is_authenticated(&self) -> bool;
}
