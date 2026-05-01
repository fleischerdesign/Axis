use axis_domain::models::continuity::Side;
use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct StartSharingUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl StartSharingUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, side: Side, edge_pos: f64) -> Result<(), ContinuityError> {
        self.provider.start_sharing(side, edge_pos).await
    }
}
