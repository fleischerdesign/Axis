use axis_domain::ports::power::{PowerError, PowerProvider};
use log::info;
use std::sync::Arc;

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
