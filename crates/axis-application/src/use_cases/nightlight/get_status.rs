use axis_domain::models::nightlight::NightlightStatus;
use axis_domain::ports::nightlight::{NightlightProvider, NightlightError};
use std::sync::Arc;

pub struct GetNightlightStatusUseCase {
    provider: Arc<dyn NightlightProvider>,
}

impl GetNightlightStatusUseCase {
    pub fn new(provider: Arc<dyn NightlightProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<NightlightStatus, NightlightError> {
        self.provider.get_status().await
    }
}
