use super::StatusStream;
use crate::models::dnd::DndStatus;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DndError {
    #[error("DND provider error: {0}")]
    ProviderError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type DndStream = StatusStream<DndStatus>;

#[async_trait]
pub trait DndProvider: Send + Sync {
    async fn get_status(&self) -> Result<DndStatus, DndError>;
    async fn subscribe(&self) -> Result<DndStream, DndError>;
    async fn set_enabled(&self, enabled: bool) -> Result<(), DndError>;
}

crate::status_provider!(DndProvider, DndStatus, DndError);
