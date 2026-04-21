use axis_domain::ports::lock::{LockProvider, LockError};
use std::sync::Arc;

pub struct AuthenticateUseCase {
    provider: Arc<dyn LockProvider>,
}

impl AuthenticateUseCase {
    pub fn new(provider: Arc<dyn LockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, password: &str) -> Result<bool, LockError> {
        self.provider.authenticate(password).await
    }
}
