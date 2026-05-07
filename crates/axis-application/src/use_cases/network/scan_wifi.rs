use axis_domain::ports::network::{NetworkError, NetworkProvider};
use log::debug;
use std::sync::Arc;

pub struct ScanWifiUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl ScanWifiUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), NetworkError> {
        debug!("[use-case] Triggering Wi-Fi scan");
        self.provider.scan_wifi().await
    }
}
