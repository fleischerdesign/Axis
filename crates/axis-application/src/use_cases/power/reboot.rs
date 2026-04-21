use axis_domain::ports::power::{PowerProvider, PowerError};
use std::sync::Arc;

pub struct RebootUseCase {
    provider: Arc<dyn PowerProvider>,
}

impl RebootUseCase {
    pub fn new(provider: Arc<dyn PowerProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), PowerError> {
        self.provider.reboot().await
    }
}
