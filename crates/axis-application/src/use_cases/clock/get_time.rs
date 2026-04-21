use axis_domain::models::clock::TimeStatus;
use axis_domain::ports::clock::{ClockProvider, ClockError};
use std::sync::Arc;

pub struct GetTimeUseCase {
    provider: Arc<dyn ClockProvider>,
}

impl GetTimeUseCase {
    pub fn new(provider: Arc<dyn ClockProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<TimeStatus, ClockError> {
        self.provider.get_time().await
    }
}
