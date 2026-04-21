use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;

pub struct SetNightlightScheduleUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SetNightlightScheduleUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, sunrise: &str, sunset: &str) -> Result<(), NightlightError> {
        self.provider.set_schedule(sunrise, sunset).await
    }
}
