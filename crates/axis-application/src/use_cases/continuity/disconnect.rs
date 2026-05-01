use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct DisconnectUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl DisconnectUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        self.provider.disconnect().await
    }
}
