use axis_domain::ports::power::{PowerProvider, PowerError};
use std::sync::Arc;
use log::info;

pub struct PowerOffUseCase {
    provider: Arc<dyn PowerProvider>,
}

impl PowerOffUseCase {
    pub fn new(provider: Arc<dyn PowerProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), PowerError> {
        info!("[use-case] System power off requested");
        self.provider.power_off().await
    }
}
