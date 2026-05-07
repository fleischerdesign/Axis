use axis_domain::ports::network::{NetworkError, NetworkProvider};
use log::info;
use std::sync::Arc;

pub struct DisconnectWifiUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl DisconnectWifiUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), NetworkError> {
        info!("[use-case] Disconnecting from Wi-Fi");
        self.provider.disconnect_wifi().await
    }
}
