use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;

pub struct SetNightlightTempNightUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SetNightlightTempNightUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, temp: u32) -> Result<(), NightlightError> {
        self.provider.set_temp_night(temp).await
    }
}
