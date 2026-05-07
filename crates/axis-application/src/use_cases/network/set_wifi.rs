use axis_domain::ports::network::{NetworkError, NetworkProvider};
use log::info;
use std::sync::Arc;

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
