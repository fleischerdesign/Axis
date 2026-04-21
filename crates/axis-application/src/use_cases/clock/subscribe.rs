use axis_domain::ports::clock::{ClockProvider, ClockError, ClockStream};
use std::sync::Arc;

pub struct SubscribeToClockUpdatesUseCase {
    provider: Arc<dyn ClockProvider>,
}

impl SubscribeToClockUpdatesUseCase {
    pub fn new(provider: Arc<dyn ClockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<ClockStream, ClockError> {
        self.provider.subscribe().await
    }
}
