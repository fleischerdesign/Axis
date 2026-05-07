use axis_domain::ports::brightness::{BrightnessError, BrightnessProvider};
use log::debug;
use std::sync::Arc;

pub struct SetBrightnessUseCase {
    provider: Arc<dyn BrightnessProvider>,
}

impl SetBrightnessUseCase {
    pub fn new(provider: Arc<dyn BrightnessProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, percentage: f64) -> Result<(), BrightnessError> {
        let percentage = percentage.clamp(0.0, 1.0);
        debug!("[use-case] Setting screen brightness to {:.2}", percentage);
        self.provider.set_brightness(percentage).await
    }
}
