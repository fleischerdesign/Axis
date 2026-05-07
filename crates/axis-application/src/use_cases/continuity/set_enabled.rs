use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider};
use log::info;
use std::sync::Arc;

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
