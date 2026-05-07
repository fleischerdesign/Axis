use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;
use log::info;

pub struct RejectPinUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl RejectPinUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        info!("[use-case] Rejecting PIN");
        self.provider.reject_pin().await
    }
}
