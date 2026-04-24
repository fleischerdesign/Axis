use axis_domain::ports::workspaces::{WorkspaceProvider, WorkspaceError};
use std::sync::Arc;
use log::info;

pub struct FocusWorkspaceUseCase {
    provider: Arc<dyn WorkspaceProvider>,
}

impl FocusWorkspaceUseCase {
    pub fn new(provider: Arc<dyn WorkspaceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: u32) -> Result<(), WorkspaceError> {
        info!("[use-case] Focusing workspace: {}", id);
        self.provider.focus_workspace(id).await
    }
}
