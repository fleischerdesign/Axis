use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;

pub struct StopBluetoothScanUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl StopBluetoothScanUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), BluetoothError> {
        self.provider.stop_scan().await
    }
}
