use axis_domain::ports::bluetooth::{BluetoothProvider, BluetoothError};
use std::sync::Arc;

pub struct PairAcceptUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl PairAcceptUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, value: Vec<u8>) -> Result<(), BluetoothError> {
        self.provider.pair_accept(value).await
    }
}
