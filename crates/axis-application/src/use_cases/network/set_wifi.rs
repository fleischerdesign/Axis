use axis_domain::ports::network::{NetworkProvider, NetworkError};
use std::sync::Arc;
use log::info;

pub struct SetWifiEnabledUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl SetWifiEnabledUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), NetworkError> {
        info!("[use-case] Setting WiFi to: {}", enabled);
        self.provider.set_wifi_enabled(enabled).await
    }
}
