use axis_domain::models::tray::TrayStatus;
use axis_domain::ports::tray::{TrayProvider, TrayError};
use std::sync::Arc;

pub struct GetTrayStatusUseCase {
    provider: Arc<dyn TrayProvider>,
}

impl GetTrayStatusUseCase {
    pub fn new(provider: Arc<dyn TrayProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<TrayStatus, TrayError> {
        self.provider.get_status().await
    }
}
