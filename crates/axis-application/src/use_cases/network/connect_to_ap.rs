use axis_domain::ports::network::{NetworkProvider, NetworkError};
use std::sync::Arc;
use log::info;

pub struct ConnectToApUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl ConnectToApUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: &str, password: Option<&str>) -> Result<(), NetworkError> {
        if id.is_empty() {
            return Err(NetworkError::ProviderError("Access point ID cannot be empty".to_string()));
        }

        info!("[use-case] Connecting to access point: {}", id);
        self.provider.connect_to_ap(id, password).await
    }
}
