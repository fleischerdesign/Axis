use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider};
use log::debug;
use std::sync::Arc;

pub struct ConnectToPeerUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl ConnectToPeerUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, peer_id: &str) -> Result<(), ContinuityError> {
        debug!("[use-case] Connecting to peer: {}", peer_id);
        self.provider.connect_to_peer(peer_id).await
    }
}
