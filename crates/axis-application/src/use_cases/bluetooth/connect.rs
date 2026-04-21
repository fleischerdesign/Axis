use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;

pub struct ConnectBluetoothDeviceUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl ConnectBluetoothDeviceUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: &str) -> Result<(), BluetoothError> {
        self.provider.connect(id).await
    }
}
