use axis_domain::models::continuity::Side;
use axis_domain::ports::continuity::{ContinuityError, ContinuitySharingProvider};
use log::debug;
use std::sync::Arc;

pub struct StartSharingUseCase {
    provider: Arc<dyn ContinuitySharingProvider>,
}

impl StartSharingUseCase {
    pub fn new(provider: Arc<dyn ContinuitySharingProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, side: Side, edge_pos: f64) -> Result<(), ContinuityError> {
        debug!("[use-case] Starting sharing");
        self.provider.start_sharing(side, edge_pos).await
    }
}
