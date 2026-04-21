use axis_domain::ports::power::{PowerProvider, PowerError, PowerStream};
use std::sync::Arc;

pub struct SubscribeToPowerUpdatesUseCase {
    provider: Arc<dyn PowerProvider>,
}

impl SubscribeToPowerUpdatesUseCase {
    pub fn new(provider: Arc<dyn PowerProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<PowerStream, PowerError> {
        self.provider.subscribe().await
    }
}
