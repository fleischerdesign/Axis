use axis_domain::ports::power::{PowerError, PowerProvider};
use log::info;
use std::sync::Arc;

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
