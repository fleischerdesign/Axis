use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError, BluetoothStream};
use std::sync::Arc;

pub struct SubscribeToBluetoothUpdatesUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl SubscribeToBluetoothUpdatesUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<BluetoothStream, BluetoothError> {
        self.provider.subscribe().await
    }
}
