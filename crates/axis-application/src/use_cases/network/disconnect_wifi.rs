use axis_domain::ports::network::{NetworkProvider, NetworkError};
use std::sync::Arc;
use log::info;

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
