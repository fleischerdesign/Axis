use axis_domain::models::continuity::PeerArrangement;
use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider};
use log::info;
use std::sync::Arc;

pub struct SetPeerArrangementUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl SetPeerArrangementUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, arrangement: PeerArrangement) -> Result<(), ContinuityError> {
        info!("[use-case] Setting peer arrangement");
        self.provider.set_peer_arrangement(arrangement).await
    }
}
