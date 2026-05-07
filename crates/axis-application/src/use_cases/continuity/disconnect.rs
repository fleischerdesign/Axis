use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider};
use log::info;
use std::sync::Arc;

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
