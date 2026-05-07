use axis_domain::ports::power::{PowerError, PowerProvider};
use log::info;
use std::sync::Arc;

pub struct RebootUseCase {
    provider: Arc<dyn PowerProvider>,
}

impl RebootUseCase {
    pub fn new(provider: Arc<dyn PowerProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), PowerError> {
        info!("[use-case] System reboot requested");
        self.provider.reboot().await
    }
}
