use axis_domain::ports::nightlight::{NightlightProvider, NightlightError, NightlightStream};
use std::sync::Arc;

pub struct SubscribeToNightlightUpdatesUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SubscribeToNightlightUpdatesUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<NightlightStream, NightlightError> {
        self.provider.subscribe().await
    }
}
