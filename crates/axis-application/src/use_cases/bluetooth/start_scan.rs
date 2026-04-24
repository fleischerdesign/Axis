use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;
use log::debug;

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
