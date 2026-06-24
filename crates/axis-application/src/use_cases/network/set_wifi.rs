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

#[cfg(test)]
mod tests {
    use super::*;
    use axis_infrastructure::mocks::network::MockNetworkProvider;

    #[tokio::test]
    async fn set_wifi_enabled() {
        let mock = MockNetworkProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetWifiEnabledUseCase::new(mock.clone());
        uc.execute(true).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!(status.is_wifi_enabled);
    }

    #[tokio::test]
    async fn set_wifi_disabled() {
        let mock = MockNetworkProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetWifiEnabledUseCase::new(mock.clone());
        uc.execute(false).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!(!status.is_wifi_enabled);
    }
}
