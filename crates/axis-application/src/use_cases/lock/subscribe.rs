use axis_domain::ports::lock::{LockProvider, LockError};
use std::sync::Arc;

pub struct SubscribeToLockUpdatesUseCase {
    provider: Arc<dyn LockProvider>,
}

impl SubscribeToLockUpdatesUseCase {
    pub fn new(provider: Arc<dyn LockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<axis_domain::ports::lock::LockStream, LockError> {
        self.provider.subscribe().await
    }
}
