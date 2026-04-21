use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError, BluetoothStatusStream};
use std::sync::Arc;

pub struct SubscribeToBluetoothUpdatesUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl SubscribeToBluetoothUpdatesUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<BluetoothStatusStream, BluetoothError> {
        self.provider.subscribe().await
    }
}
