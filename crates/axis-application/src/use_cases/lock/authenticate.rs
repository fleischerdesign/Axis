use axis_domain::ports::lock::{LockProvider, LockError};
use std::sync::Arc;
use log::{info, warn};

pub struct AuthenticateUseCase {
    provider: Arc<dyn LockProvider>,
}

impl AuthenticateUseCase {
    pub fn new(provider: Arc<dyn LockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, password: &str) -> Result<bool, LockError> {
        let result = self.provider.authenticate(password).await;
        
        match &result {
            Ok(true) => info!("[use-case] Authentication successful"),
            Ok(false) => warn!("[use-case] Authentication failed: invalid password"),
            Err(e) => warn!("[use-case] Authentication error: {}", e),
        }
        
        result
    }
}
