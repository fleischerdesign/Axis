use axis_domain::ports::network::{NetworkProvider, NetworkError};
use std::sync::Arc;

pub struct ScanWifiUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl ScanWifiUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), NetworkError> {
        self.provider.scan_wifi().await
    }
}
