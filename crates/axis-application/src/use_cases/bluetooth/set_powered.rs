use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;

pub struct SetBluetoothPoweredUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl SetBluetoothPoweredUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, powered: bool) -> Result<(), BluetoothError> {
        self.provider.set_powered(powered).await
    }
}
