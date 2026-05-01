use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct ConfirmPinUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl ConfirmPinUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        self.provider.confirm_pin().await
    }
}
