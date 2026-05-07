use axis_domain::ports::workspaces::{WorkspaceError, WorkspaceProvider};
use log::info;
use std::sync::Arc;

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
