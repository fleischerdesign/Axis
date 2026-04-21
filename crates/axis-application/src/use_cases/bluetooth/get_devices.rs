use axis_domain::models::bluetooth::BluetoothDevice;
use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;

pub struct GetBluetoothDevicesUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl GetBluetoothDevicesUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<Vec<BluetoothDevice>, BluetoothError> {
        self.provider.get_devices().await
    }
}
