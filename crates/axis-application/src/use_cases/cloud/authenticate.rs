use axis_domain::ports::cloud_auth::{CloudAuthProvider, AuthError};
use axis_domain::ports::cloud::{CloudProvider, CloudError};
use std::sync::Arc;

pub struct AuthenticateAccountUseCase {
    auth_provider: Arc<dyn CloudAuthProvider>,
    cloud_provider: Arc<dyn CloudProvider>,
}

impl AuthenticateAccountUseCase {
    pub fn new(
        auth_provider: Arc<dyn CloudAuthProvider>,
        cloud_provider: Arc<dyn CloudProvider>,
    ) -> Self {
        Self { auth_provider, cloud_provider }
    }

    pub async fn execute(&self, scopes: Vec<String>) -> Result<(), AuthError> {
        let account = self.auth_provider.authenticate(&scopes).await?;
        
        self.cloud_provider.add_account(account).await
            .map_err(|e| AuthError::Failed(format!("Failed to store account: {}", e)))?;
            
        Ok(())
    }
}
