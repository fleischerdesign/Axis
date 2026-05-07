use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider};
use log::info;
use std::sync::Arc;

pub struct ConfirmPinUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl ConfirmPinUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        info!("[use-case] Confirming PIN");
        self.provider.confirm_pin().await
    }
}
