use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;
use log::info;

pub struct CancelReconnectUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl CancelReconnectUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        info!("[use-case] Cancelling reconnect");
        self.provider.cancel_reconnect().await
    }
}
