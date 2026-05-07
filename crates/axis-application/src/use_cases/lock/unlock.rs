use axis_domain::ports::lock::{LockError, LockProvider};
use log::info;
use std::sync::Arc;

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
