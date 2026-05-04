use crate::models::ssh::SshStatus;
use async_trait::async_trait;
use thiserror::Error;

use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum SshError {
    #[error("SSH provider error: {0}")]
    ProviderError(String),
}

pub type SshStream = StatusStream<SshStatus>;

#[async_trait]
pub trait SshProvider: Send + Sync {
    async fn get_status(&self) -> Result<SshStatus, SshError>;
    async fn subscribe(&self) -> Result<SshStream, SshError>;
}

crate::status_provider!(SshProvider, SshStatus, SshError);
