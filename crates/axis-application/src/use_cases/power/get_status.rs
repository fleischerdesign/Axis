use axis_domain::models::power::PowerStatus;
use axis_domain::ports::power::{PowerProvider, PowerError};
use std::sync::Arc;

pub struct GetPowerStatusUseCase {
    provider: Arc<dyn PowerProvider>,
}

impl GetPowerStatusUseCase {
    pub fn new(provider: Arc<dyn PowerProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<PowerStatus, PowerError> {
        self.provider.get_status().await
    }
}
