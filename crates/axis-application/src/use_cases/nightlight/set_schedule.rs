use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;
use log::info;

pub struct SetNightlightScheduleUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SetNightlightScheduleUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, sunrise: &str, sunset: &str) -> Result<(), NightlightError> {
        info!("[use-case] Updating nightlight schedule: sunrise={}, sunset={}", sunrise, sunset);
        self.provider.set_schedule(sunrise, sunset).await
    }
}
