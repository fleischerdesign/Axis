use axis_domain::ports::cloud_auth::{CloudAuthProvider, AuthError};
use std::sync::Arc;

pub struct AuthenticateAccountUseCase {
    auth_provider: Arc<dyn CloudAuthProvider>,
}

impl AuthenticateAccountUseCase {
    pub fn new(auth_provider: Arc<dyn CloudAuthProvider>) -> Self {
        Self { auth_provider }
    }

    pub async fn execute(&self, scopes: Vec<String>) -> Result<(), AuthError> {
        self.auth_provider.authenticate(&scopes).await
    }
}
