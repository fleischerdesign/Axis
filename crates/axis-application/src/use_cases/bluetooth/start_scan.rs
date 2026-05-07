use axis_domain::ports::bluetooth::{BluetoothError, BluetoothProvider};
use log::debug;
use std::sync::Arc;

pub struct StartBluetoothScanUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl StartBluetoothScanUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), BluetoothError> {
        debug!("[use-case] Starting Bluetooth discovery scan");
        self.provider.start_scan().await
    }
}
