use axis_domain::ports::power::{PowerProvider, PowerError};
use std::sync::Arc;

pub struct PowerOffUseCase {
    provider: Arc<dyn PowerProvider>,
}

impl PowerOffUseCase {
    pub fn new(provider: Arc<dyn PowerProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), PowerError> {
        self.provider.power_off().await
    }
}
