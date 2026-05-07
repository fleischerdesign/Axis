use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;
use log::info;

pub struct ConnectToPeerUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl ConnectToPeerUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, peer_id: &str) -> Result<(), ContinuityError> {
        info!("[use-case] Connecting to peer: {}", peer_id);
        self.provider.connect_to_peer(peer_id).await
    }
}
