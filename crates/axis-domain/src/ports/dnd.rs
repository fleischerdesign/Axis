use crate::models::dnd::DndStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug)]
pub enum DndError {
    #[error("DND provider error: {0}")]
    ProviderError(String),
}

pub type DndStream = StatusStream<DndStatus>;

#[async_trait]
pub trait DndProvider: Send + Sync {
    async fn get_status(&self) -> Result<DndStatus, DndError>;
    async fn subscribe(&self) -> Result<DndStream, DndError>;
    async fn set_enabled(&self, enabled: bool) -> Result<(), DndError>;
}
