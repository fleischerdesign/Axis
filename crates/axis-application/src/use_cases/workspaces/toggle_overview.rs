use axis_domain::ports::workspaces::{WorkspaceProvider, WorkspaceError};
use std::sync::Arc;
use log::info;

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
