use axis_domain::ports::lock::{LockProvider, LockError};
use std::sync::Arc;
use log::info;

pub struct UnlockSessionUseCase {
    provider: Arc<dyn LockProvider>,
}

impl UnlockSessionUseCase {
    pub fn new(provider: Arc<dyn LockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), LockError> {
        info!("[use-case] Unlocking session");
        self.provider.unlock().await
    }
}
