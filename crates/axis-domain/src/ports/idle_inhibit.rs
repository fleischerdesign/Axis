use crate::models::idle_inhibit::IdleInhibitStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum IdleInhibitError {
    #[error("Idle inhibit provider error: {0}")]
    ProviderError(String),
}

pub type IdleInhibitStream = StatusStream<IdleInhibitStatus>;

#[async_trait]
pub trait IdleInhibitProvider: Send + Sync {
    async fn get_status(&self) -> Result<IdleInhibitStatus, IdleInhibitError>;
    async fn subscribe(&self) -> Result<IdleInhibitStream, IdleInhibitError>;
    async fn set_inhibited(&self, inhibited: bool) -> Result<(), IdleInhibitError>;
}

crate::status_provider!(IdleInhibitProvider, IdleInhibitStatus, IdleInhibitError);
