use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;
use log::info;

pub struct SetNightlightEnabledUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SetNightlightEnabledUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), NightlightError> {
        info!("[use-case] Setting nightlight enabled to: {}", enabled);
        self.provider.set_enabled(enabled).await
    }
}
