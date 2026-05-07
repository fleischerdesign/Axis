use axis_domain::ports::workspaces::{WorkspaceError, WorkspaceProvider};
use log::info;
use std::sync::Arc;

pub struct ToggleOverviewUseCase {
    provider: Arc<dyn WorkspaceProvider>,
}

impl ToggleOverviewUseCase {
    pub fn new(provider: Arc<dyn WorkspaceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), WorkspaceError> {
        info!("[use-case] Toggling Niri overview");
        self.provider.toggle_overview().await
    }
}
