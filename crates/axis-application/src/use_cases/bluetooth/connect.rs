use axis_domain::ports::bluetooth::{BluetoothError, BluetoothProvider};
use log::info;
use std::sync::Arc;

pub struct ConnectBluetoothDeviceUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl ConnectBluetoothDeviceUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: &str) -> Result<(), BluetoothError> {
        if id.is_empty() {
            return Err(BluetoothError::ValidationError(
                "Bluetooth device ID cannot be empty".to_string(),
            ));
        }

        info!(
            "[use-case] Attempting to connect to Bluetooth device: {}",
            id
        );
        self.provider.connect(id).await
    }
}
