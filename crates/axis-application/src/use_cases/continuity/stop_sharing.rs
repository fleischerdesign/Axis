use axis_domain::ports::continuity::{ContinuitySharingProvider, ContinuityError};
use std::sync::Arc;

pub struct StopSharingUseCase {
    provider: Arc<dyn ContinuitySharingProvider>,
}

impl StopSharingUseCase {
    pub fn new(provider: Arc<dyn ContinuitySharingProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, edge_pos: f64) -> Result<(), ContinuityError> {
        self.provider.stop_sharing(edge_pos).await
    }
}
