use axis_domain::models::continuity::Peer;
use axis_domain::ports::continuity::{PeerDiscovery, ContinuityError};
use std::sync::Arc;
use log::debug;

pub struct DiscoverPeersUseCase {
    discovery: Arc<dyn PeerDiscovery>,
}

impl DiscoverPeersUseCase {
    pub fn new(discovery: Arc<dyn PeerDiscovery>) -> Self {
        Self { discovery }
    }

    pub async fn execute(&self) -> Result<Vec<Peer>, ContinuityError> {
        debug!("[use-case] Refreshing discovered peers list");
        let peers = self.discovery.get_discovered_peers().await?;
        debug!("[use-case] Found {} active peers", peers.len());
        Ok(peers)
    }
}
