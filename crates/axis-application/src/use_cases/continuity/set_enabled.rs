use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct SetContinuityEnabledUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl SetContinuityEnabledUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), ContinuityError> {
        self.provider.set_enabled(enabled).await
    }
}
