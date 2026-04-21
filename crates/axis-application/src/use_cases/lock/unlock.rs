use axis_domain::ports::lock::{LockProvider, LockError};
use std::sync::Arc;

pub struct UnlockSessionUseCase {
    provider: Arc<dyn LockProvider>,
}

impl UnlockSessionUseCase {
    pub fn new(provider: Arc<dyn LockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), LockError> {
        self.provider.unlock().await
    }
}
