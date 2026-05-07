use axis_domain::ports::lock::{LockError, LockProvider};
use log::info;
use std::sync::Arc;

pub struct LockSessionUseCase {
    provider: Arc<dyn LockProvider>,
}

impl LockSessionUseCase {
    pub fn new(provider: Arc<dyn LockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), LockError> {
        info!("[use-case] Locking session");
        self.provider.lock().await
    }
}
