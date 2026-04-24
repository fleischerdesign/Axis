use axis_domain::ports::lock::{LockProvider, LockError};
use std::sync::Arc;
use log::info;

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
