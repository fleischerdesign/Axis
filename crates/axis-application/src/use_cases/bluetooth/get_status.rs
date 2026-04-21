use axis_domain::models::bluetooth::BluetoothStatus;
use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;

pub struct GetBluetoothStatusUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl GetBluetoothStatusUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<BluetoothStatus, BluetoothError> {
        self.provider.get_status().await
    }
}
