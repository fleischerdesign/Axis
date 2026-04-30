use crate::models::workspaces::WorkspaceStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum WorkspaceError {
    #[error("Workspace provider error: {0}")]
    ProviderError(String),
}

pub type WorkspaceStream = StatusStream<WorkspaceStatus>;

#[async_trait]
pub trait WorkspaceProvider: Send + Sync {
    async fn get_status(&self) -> Result<WorkspaceStatus, WorkspaceError>;
    async fn subscribe(&self) -> Result<WorkspaceStream, WorkspaceError>;
    async fn focus_workspace(&self, id: u32) -> Result<(), WorkspaceError>;
}

crate::status_provider!(WorkspaceProvider, WorkspaceStatus, WorkspaceError);
