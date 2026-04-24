use axis_domain::ports::power::{PowerProvider, PowerError};
use std::sync::Arc;
use log::info;

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
