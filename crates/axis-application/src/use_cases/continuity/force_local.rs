use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct ForceLocalUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl ForceLocalUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        self.provider.force_local().await
    }
}
