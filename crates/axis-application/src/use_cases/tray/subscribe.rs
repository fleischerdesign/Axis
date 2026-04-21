use axis_domain::ports::tray::{TrayProvider, TrayError, TrayStream};
use std::sync::Arc;

pub struct SubscribeToTrayUpdatesUseCase {
    provider: Arc<dyn TrayProvider>,
}

impl SubscribeToTrayUpdatesUseCase {
    pub fn new(provider: Arc<dyn TrayProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<TrayStream, TrayError> {
        self.provider.subscribe().await
    }
}
