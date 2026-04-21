use axis_domain::ports::brightness::{BrightnessProvider, BrightnessError};
use std::sync::Arc;

pub struct SetBrightnessUseCase {
    provider: Arc<dyn BrightnessProvider>,
}

impl SetBrightnessUseCase {
    pub fn new(provider: Arc<dyn BrightnessProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, percentage: f64) -> Result<(), BrightnessError> {
        self.provider.set_brightness(percentage).await
    }
}
