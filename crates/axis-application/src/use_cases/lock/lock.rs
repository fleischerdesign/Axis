use axis_domain::ports::lock::{LockProvider, LockError};
use std::sync::Arc;

pub struct LockSessionUseCase {
    provider: Arc<dyn LockProvider>,
}

impl LockSessionUseCase {
    pub fn new(provider: Arc<dyn LockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), LockError> {
        self.provider.lock().await
    }
}
