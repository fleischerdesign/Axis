use axis_domain::ports::power::{PowerProvider, PowerError};
use std::sync::Arc;

pub struct SuspendUseCase {
    provider: Arc<dyn PowerProvider>,
}

impl SuspendUseCase {
    pub fn new(provider: Arc<dyn PowerProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), PowerError> {
        self.provider.suspend().await
    }
}
