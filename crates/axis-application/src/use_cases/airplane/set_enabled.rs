use axis_domain::ports::airplane::{AirplaneProvider, AirplaneError};
use std::sync::Arc;
use log::info;

pub struct SetAirplaneModeUseCase {
    provider: Arc<dyn AirplaneProvider>,
}

impl SetAirplaneModeUseCase {
    pub fn new(provider: Arc<dyn AirplaneProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), AirplaneError> {
        info!("[use-case] Setting airplane mode to: {}", enabled);
        self.provider.set_enabled(enabled).await
    }
}
