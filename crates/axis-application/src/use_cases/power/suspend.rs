use axis_domain::ports::power::{PowerProvider, PowerError};
use std::sync::Arc;
use log::info;

pub struct SuspendUseCase {
    provider: Arc<dyn PowerProvider>,
}

impl SuspendUseCase {
    pub fn new(provider: Arc<dyn PowerProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), PowerError> {
        info!("[use-case] System suspend requested");
        self.provider.suspend().await
    }
}
