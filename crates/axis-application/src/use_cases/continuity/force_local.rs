use axis_domain::ports::continuity::{ContinuitySharingProvider, ContinuityError};
use std::sync::Arc;
use log::info;

pub struct ForceLocalUseCase {
    provider: Arc<dyn ContinuitySharingProvider>,
}

impl ForceLocalUseCase {
    pub fn new(provider: Arc<dyn ContinuitySharingProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<(), ContinuityError> {
        info!("[use-case] Forcing local");
        self.provider.force_local().await
    }
}
