use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;
use log::info;

pub struct DisconnectUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl DisconnectUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        info!("[use-case] Disconnecting from peer");
        self.provider.disconnect().await
    }
}
