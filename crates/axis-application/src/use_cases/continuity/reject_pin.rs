use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct RejectPinUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl RejectPinUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        self.provider.reject_pin().await
    }
}
