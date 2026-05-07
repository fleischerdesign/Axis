use axis_domain::ports::bluetooth::{BluetoothError, BluetoothProvider};
use std::sync::Arc;

pub struct PairRejectUseCase {
    provider: Arc<dyn BluetoothProvider>,
}

impl PairRejectUseCase {
    pub fn new(provider: Arc<dyn BluetoothProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), BluetoothError> {
        self.provider.pair_reject().await
    }
}
