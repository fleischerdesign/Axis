use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;
use log::debug;

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
