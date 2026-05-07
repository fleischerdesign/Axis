use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;
use log::info;

pub struct SetContinuityEnabledUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl SetContinuityEnabledUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), ContinuityError> {
        info!("[use-case] Setting continuity enabled: {}", enabled);
        self.provider.set_enabled(enabled).await
    }
}
