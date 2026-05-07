use axis_domain::models::continuity::Side;
use axis_domain::ports::continuity::{ContinuitySharingProvider, ContinuityError};
use std::sync::Arc;
use log::info;

pub struct StartSharingUseCase {
    provider: Arc<dyn ContinuitySharingProvider>,
}

impl StartSharingUseCase {
    pub fn new(provider: Arc<dyn ContinuitySharingProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, side: Side, edge_pos: f64) -> Result<(), ContinuityError> {
        info!("[use-case] Starting sharing");
        self.provider.start_sharing(side, edge_pos).await
    }
}
