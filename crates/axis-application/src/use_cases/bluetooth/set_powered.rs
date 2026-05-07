use axis_domain::ports::bluetooth::{BluetoothError, BluetoothProvider};
use log::info;
use std::sync::Arc;

pub struct SetBluetoothPoweredUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl SetBluetoothPoweredUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, powered: bool) -> Result<(), BluetoothError> {
        info!(
            "[use-case] Setting Bluetooth adapter powered state to: {}",
            powered
        );
        self.provider.set_powered(powered).await
    }
}
