use axis_domain::ports::brightness::{BrightnessProvider, BrightnessError};
use std::sync::Arc;
use log::debug;

pub struct SetBrightnessUseCase {
    provider: Arc<dyn BrightnessProvider>,
}

impl SetBrightnessUseCase {
    pub fn new(provider: Arc<dyn BrightnessProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, percentage: f64) -> Result<(), BrightnessError> {
        let percentage = percentage.clamp(0.0, 100.0);
        debug!("[use-case] Setting screen brightness to {:.0}%", percentage * 100.0);
        self.provider.set_brightness(percentage).await
    }
}
