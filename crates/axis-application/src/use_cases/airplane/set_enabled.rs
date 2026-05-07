use axis_domain::ports::airplane::{AirplaneError, AirplaneProvider};
use log::info;
use std::sync::Arc;

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
