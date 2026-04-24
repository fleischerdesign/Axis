use axis_domain::ports::cloud_auth::{CloudAuthProvider, AuthError};
use axis_domain::ports::cloud::CloudProvider;
use std::sync::Arc;
use log::{info, error};

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
        info!("[use-case] Starting cloud authentication flow with {} scopes", scopes.len());

        let account = match self.auth_provider.authenticate(&scopes).await {
            Ok(acc) => {
                info!("[use-case] Authentication successful for account: {}", acc.display_name);
                acc
            },
            Err(e) => {
                error!("[use-case] Cloud authentication failed: {}", e);
                return Err(e);
            }
        };
        
        self.cloud_provider.add_account(account).await
            .map_err(|e| {
                let err_msg = format!("Failed to store account: {}", e);
                error!("[use-case] {}", err_msg);
                AuthError::Failed(err_msg)
            })?;
            
        Ok(())
    }
}
