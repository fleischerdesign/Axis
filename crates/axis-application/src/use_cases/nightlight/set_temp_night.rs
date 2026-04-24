use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;
use log::debug;

pub struct SetNightlightTempNightUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SetNightlightTempNightUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, temp: u32) -> Result<(), NightlightError> {
        let temp = temp.clamp(1000, 10000);
        debug!("[use-case] Setting nightlight night temperature to {}K", temp);
        self.provider.set_temp_night(temp).await
    }
}
