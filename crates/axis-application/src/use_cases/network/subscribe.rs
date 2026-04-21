use axis_domain::ports::network::{NetworkProvider, NetworkError, NetworkStream};
use std::sync::Arc;

pub struct SubscribeToNetworkUpdatesUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl SubscribeToNetworkUpdatesUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<NetworkStream, NetworkError> {
        self.provider.subscribe().await
    }
}
