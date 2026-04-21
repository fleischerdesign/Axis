use axis_domain::models::brightness::BrightnessStatus;
use axis_domain::ports::brightness::{BrightnessProvider, BrightnessError};
use std::sync::Arc;

pub struct GetBrightnessStatusUseCase {
    provider: Arc<dyn BrightnessProvider>,
}

impl GetBrightnessStatusUseCase {
    pub fn new(provider: Arc<dyn BrightnessProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<BrightnessStatus, BrightnessError> {
        self.provider.get_status().await
    }
}
