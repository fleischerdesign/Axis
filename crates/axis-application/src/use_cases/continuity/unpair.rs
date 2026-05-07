use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;
use log::info;

pub struct UnpairUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl UnpairUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, peer_id: &str) -> Result<(), ContinuityError> {
        info!("[use-case] Unpairing peer: {}", peer_id);
        self.provider.unpair(peer_id).await
    }
}
