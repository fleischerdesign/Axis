use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;

pub struct StartBluetoothScanUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl StartBluetoothScanUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), BluetoothError> {
        self.provider.start_scan().await
    }
}
