use axis_domain::models::airplane::AirplaneStatus;
use axis_domain::ports::airplane::{AirplaneProvider, AirplaneError};
use std::sync::Arc;

pub struct GetAirplaneStatusUseCase {
    provider: Arc<dyn AirplaneProvider>,
}

impl GetAirplaneStatusUseCase {
    pub fn new(provider: Arc<dyn AirplaneProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<AirplaneStatus, AirplaneError> {
        self.provider.get_status().await
    }
}
