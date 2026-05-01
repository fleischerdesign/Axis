use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct UnpairUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl UnpairUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, peer_id: &str) -> Result<(), ContinuityError> {
        self.provider.unpair(peer_id).await
    }
}
