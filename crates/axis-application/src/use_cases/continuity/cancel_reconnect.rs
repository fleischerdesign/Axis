use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct CancelReconnectUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl CancelReconnectUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        self.provider.cancel_reconnect().await
    }
}
