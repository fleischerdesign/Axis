use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;
use log::debug;

pub struct SetNightlightTempDayUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl SetNightlightTempDayUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, temp: u32) -> Result<(), NightlightError> {
        let temp = temp.clamp(1000, 10000);
        debug!("[use-case] Setting nightlight day temperature to {}K", temp);
        self.provider.set_temp_day(temp).await
    }
}
