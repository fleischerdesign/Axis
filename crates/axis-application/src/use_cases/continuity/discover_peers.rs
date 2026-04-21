use axis_domain::models::continuity::Peer;
use axis_domain::ports::continuity::{PeerDiscovery, ContinuityError};
use std::sync::Arc;

pub struct DiscoverPeersUseCase {
    discovery: Arc<dyn PeerDiscovery>,
}

impl DiscoverPeersUseCase {
    pub fn new(discovery: Arc<dyn PeerDiscovery>) -> Self {
        Self { discovery }
    }

    pub async fn execute(&self) -> Result<Vec<Peer>, ContinuityError> {
        self.discovery.get_discovered_peers().await
    }
}
