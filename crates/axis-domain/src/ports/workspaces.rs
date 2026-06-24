use super::StatusStream;
use crate::models::workspaces::WorkspaceStatus;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum WorkspaceError {
    #[error("Workspace provider error: {0}")]
    ProviderError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type WorkspaceStream = StatusStream<WorkspaceStatus>;

/// Provides Niri workspace state and allows workspace focus/overview toggling.
#[async_trait]
pub trait WorkspaceProvider: Send + Sync {
    async fn get_status(&self) -> Result<WorkspaceStatus, WorkspaceError>;
    async fn subscribe(&self) -> Result<WorkspaceStream, WorkspaceError>;
    async fn focus_workspace(&self, id: u32) -> Result<(), WorkspaceError>;
    async fn toggle_overview(&self) -> Result<(), WorkspaceError>;
}

crate::status_provider!(WorkspaceProvider, WorkspaceStatus, WorkspaceError);
