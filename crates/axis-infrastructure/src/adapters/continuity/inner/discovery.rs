use axis_domain::models::continuity::SharingState;
use log::info;

use super::super::connection::{ConnectionEvent, ConnectionProvider, TcpConnectionProvider};
use super::super::discovery::DiscoveryEvent;
use super::ContinuityInner;

impl ContinuityInner {
    pub(crate) async fn handle_discovery_event_with_conn(
        &mut self,
        event: DiscoveryEvent,
        connection: &mut TcpConnectionProvider,
        conn_tx: &async_channel::Sender<ConnectionEvent>,
    ) {
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

                let peer_id = peer.device_id.clone();
                let peer_name = peer.device_name.clone();

                let (addr_v4, addr_v6) = if let Some(existing) = self
                    .status
                    .peers
                    .iter_mut()
                    .find(|p| p.device_id == peer_id)
                {
                    existing.device_name = peer.device_name.clone();
                    existing.hostname = peer.hostname.clone();
                    if peer.address.is_ipv6() {
                        existing.address_v6 = Some(peer.address);
                    } else {
                        existing.address = peer.address;
                    }
                    if peer.address_v6.is_some() {
                        existing.address_v6 = peer.address_v6;
                    }
                    (existing.address, existing.address_v6)
                } else {
                    let v4 = peer.address;
                    let v6 = peer.address_v6;
                    info!("[continuity] peer found: {} at {}", peer_name, v4);
                    self.status.peers.push(peer);
                    (v4, v6)
                };

                if self.status.active_connection.is_none()
                    && !self.is_initiating
                    && let Some(config) = self.status.peer_configs.get(&peer_id)
                    && config.trusted
                    && config.auto_connect
                {
                    info!("[continuity] auto-connecting to trusted peer {}", peer_name);
                    self.is_initiating = true;
                    self.pending_peer = Some((peer_id.clone(), peer_name.clone()));

                    connection.connect_dual(
                        addr_v4,
                        addr_v6,
                        conn_tx.clone(),
                        self.status.device_id.clone(),
                        self.status.device_name.clone(),
                    );
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
