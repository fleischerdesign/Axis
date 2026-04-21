use axis_domain::ports::workspaces::{WorkspaceProvider, WorkspaceError, WorkspaceStream};
use std::sync::Arc;

pub struct SubscribeToWorkspaceUpdatesUseCase {
    provider: Arc<dyn WorkspaceProvider>,
}

impl SubscribeToWorkspaceUpdatesUseCase {
    pub fn new(provider: Arc<dyn WorkspaceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<WorkspaceStream, WorkspaceError> {
        self.provider.subscribe().await
    }
}
