use axis_domain::ports::bluetooth::{BluetoothError, BluetoothProvider};
use log::info;
use std::sync::Arc;

pub struct DisconnectBluetoothDeviceUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl DisconnectBluetoothDeviceUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: &str) -> Result<(), BluetoothError> {
        info!("[use-case] Disconnecting from Bluetooth device: {}", id);
        self.provider.disconnect(id).await
    }
}
