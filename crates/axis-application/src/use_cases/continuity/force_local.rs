use axis_domain::ports::continuity::{ContinuitySharingProvider, ContinuityError};
use std::sync::Arc;

pub struct ForceLocalUseCase {
    provider: Arc<dyn ContinuitySharingProvider>,
}

impl ForceLocalUseCase {
    pub fn new(provider: Arc<dyn ContinuitySharingProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        self.provider.force_local().await
    }
}
