use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;

pub struct SetNightlightTempDayUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SetNightlightTempDayUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, temp: u32) -> Result<(), NightlightError> {
        self.provider.set_temp_day(temp).await
    }
}
