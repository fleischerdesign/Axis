use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;
use log::info;

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
