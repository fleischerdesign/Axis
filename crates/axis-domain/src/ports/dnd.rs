use crate::models::dnd::DndStatus;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum DndError {
    #[error("DND provider error: {0}")]
    ProviderError(String),
}

pub type DndStream = Pin<Box<dyn Stream<Item = DndStatus> + Send>>;

#[async_trait]
pub trait DndProvider: Send + Sync {
    async fn get_status(&self) -> Result<DndStatus, DndError>;
    async fn subscribe(&self) -> Result<DndStream, DndError>;
    async fn set_enabled(&self, enabled: bool) -> Result<(), DndError>;
}
