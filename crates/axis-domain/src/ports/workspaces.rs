use crate::models::workspaces::WorkspaceStatus;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Workspace provider error: {0}")]
    ProviderError(String),
}

pub type WorkspaceStream = Pin<Box<dyn Stream<Item = WorkspaceStatus> + Send>>;

#[async_trait]
pub trait WorkspaceProvider: Send + Sync {
    async fn get_status(&self) -> Result<WorkspaceStatus, WorkspaceError>;
    async fn subscribe(&self) -> Result<WorkspaceStream, WorkspaceError>;
    /// Schaltet den Fokus auf einen bestimmten Workspace
    async fn focus_workspace(&self, id: u32) -> Result<(), WorkspaceError>;
}
