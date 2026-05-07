use axis_domain::ports::bluetooth::{BluetoothError, BluetoothProvider};
use log::debug;
use std::sync::Arc;

pub struct StopBluetoothScanUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl StopBluetoothScanUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), BluetoothError> {
        debug!("[use-case] Stopping Bluetooth discovery scan");
        self.provider.stop_scan().await
    }
}
