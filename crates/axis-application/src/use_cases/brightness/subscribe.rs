use axis_domain::ports::brightness::{BrightnessProvider, BrightnessError, BrightnessStream};
use std::sync::Arc;

pub struct SubscribeToBrightnessUpdatesUseCase {
    provider: Arc<dyn BrightnessProvider>,
}

impl SubscribeToBrightnessUpdatesUseCase {
    pub fn new(provider: Arc<dyn BrightnessProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<BrightnessStream, BrightnessError> {
        self.provider.subscribe().await
    }
}
