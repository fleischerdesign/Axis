use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct StopSharingUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl StopSharingUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, edge_pos: f64) -> Result<(), ContinuityError> {
        self.provider.stop_sharing(edge_pos).await
    }
}
