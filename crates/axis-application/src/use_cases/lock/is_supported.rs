use axis_domain::ports::lock::{LockProvider, LockError};
use std::sync::Arc;

pub struct IsLockSupportedUseCase {
    provider: Arc<dyn LockProvider>,
}

impl IsLockSupportedUseCase {
    pub fn new(provider: Arc<dyn LockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<bool, LockError> {
        self.provider.is_supported().await
    }
}
