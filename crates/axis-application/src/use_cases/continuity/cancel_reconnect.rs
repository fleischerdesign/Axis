use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider};
use log::info;
use std::sync::Arc;

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
