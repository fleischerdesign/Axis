use axis_domain::models::network::NetworkStatus;
use axis_domain::ports::network::{NetworkProvider, NetworkError};
use std::sync::Arc;

pub struct GetNetworkStatusUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl GetNetworkStatusUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<NetworkStatus, NetworkError> {
        self.provider.get_status().await
    }
}
