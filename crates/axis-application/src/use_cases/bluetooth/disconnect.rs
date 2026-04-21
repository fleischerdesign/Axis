use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;

pub struct DisconnectBluetoothDeviceUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl DisconnectBluetoothDeviceUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: &str) -> Result<(), BluetoothError> {
        self.provider.disconnect(id).await
    }
}
