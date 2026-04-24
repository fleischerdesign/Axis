use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;
use log::info;

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
