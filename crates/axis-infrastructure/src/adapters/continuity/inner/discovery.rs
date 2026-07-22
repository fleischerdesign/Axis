use axis_domain::models::continuity::SharingState;
use log::info;

use super::super::discovery::DiscoveryEvent;
use super::ContinuityInner;

impl ContinuityInner {
    pub(crate) async fn handle_discovery_event(&mut self, event: DiscoveryEvent) {
        match event {
            DiscoveryEvent::PeerFound(peer) => {
                let peer_host = peer.hostname.trim_end_matches(".local");
                let my_host = self.status.device_name.trim_end_matches(".local");
                let my_id = &self.status.device_id;
                if peer_host.eq_ignore_ascii_case(my_host)
                    || peer
                        .device_name
                        .trim_end_matches(".local")
                        .eq_ignore_ascii_case(my_host)
                    || &peer.device_id == my_id
                {
                    return;
                }
                if let Some(existing) = self
                    .status
                    .peers
                    .iter_mut()
                    .find(|p| p.device_id == peer.device_id)
                {
                    *existing = peer;
                } else {
                    info!(
                        "[continuity] peer found: {} at {}",
                        peer.device_name, peer.address
                    );
                    self.status.peers.push(peer);
                }
                self.push();
            }
            DiscoveryEvent::PeerLost(device_id) => {
                self.status.peers.retain(|p| p.device_id != device_id);
                if self
                    .status
                    .active_connection
                    .as_ref()
                    .is_some_and(|c| c.peer_id == device_id)
                {
                    info!("[continuity] active peer lost");
                    self.status.active_connection = None;
                    self.connected_at = None;
                    self.status.sharing_state = SharingState::Idle;
                    self.last_message_at = None;
                }
                self.push();
            }
        }
    }
}
