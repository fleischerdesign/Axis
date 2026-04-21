use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;

pub struct SetNightlightEnabledUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SetNightlightEnabledUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), NightlightError> {
        self.provider.set_enabled(enabled).await
    }
}
