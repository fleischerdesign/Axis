use axis_domain::ports::network::{NetworkProvider, NetworkError};
use std::sync::Arc;

pub struct DisconnectWifiUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl DisconnectWifiUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), NetworkError> {
        self.provider.disconnect_wifi().await
    }
}
