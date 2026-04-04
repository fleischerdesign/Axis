use log::{info, warn};

use super::{ContinuityInner, SharingState};
use super::connection::{ConnectionProvider, TcpConnectionProvider};
use super::discovery::DiscoveryEvent;
use super::input::{EvdevCapture, InputCapture};
use std::time::Duration;

impl ContinuityInner {
    pub(super) async fn handle_discovery_event(&mut self, event: DiscoveryEvent) {
        match event {
            DiscoveryEvent::PeerFound(peer) => {
                if peer.hostname == self.data.device_name {
                    return;
                }
                if !self.data.peers.iter().any(|p| p.device_id == peer.device_id) {
                    info!("[continuity] peer found: {} at {}", peer.device_name, peer.address);
                    self.data.peers.push(peer);
                    self.push();
                }
            }
            DiscoveryEvent::PeerLost(device_id) => {
                self.data.peers.retain(|p| p.device_id != device_id);
                if self
                    .data
                    .active_connection
                    .as_ref()
                    .is_some_and(|c| c.peer_id == device_id)
                {
                    info!("[continuity] active peer lost");
                    self.data.active_connection = None;
                    self.data.sharing_state = SharingState::Idle;
                    self.last_message_at = None;
                }
                self.push();
            }
        }
    }

    pub(super) fn handle_heartbeat(&mut self, connection: &mut TcpConnectionProvider, capture: &mut EvdevCapture) {
        if let Some(last) = self.last_message_at {
            let timeout = if matches!(self.data.sharing_state, SharingState::Receiving) {
                Duration::from_secs(5)
            } else if matches!(&self.data.sharing_state, SharingState::Pending { .. } | SharingState::PendingSwitch) {
                Duration::from_secs(5)
            } else {
                Duration::from_secs(super::CONNECTION_TIMEOUT_SECS)
            };

            if last.elapsed() > timeout {
                warn!("[continuity] peer timed out (no message for {:?})", timeout);
                connection.disconnect_active();
                capture.stop();
                self.data.active_connection = None;
                self.data.sharing_state = SharingState::Idle;
                self.last_message_at = None;
                self.push();
                return;
            }
        }

        connection.send_message(super::protocol::Message::Heartbeat);

        if let Some(pin) = &self.data.pending_pin {
            if pin.created_at.elapsed() > Duration::from_secs(super::PIN_EXPIRY_SECS) {
                warn!("[continuity] PIN expired ({}s timeout)", super::PIN_EXPIRY_SECS);
                self.data.pending_pin = None;
                connection.send_message(super::protocol::Message::Disconnect {
                    reason: "PIN expired".to_string(),
                });
                connection.disconnect_active();
                self.push();
            }
        }
    }

    pub(super) async fn handle_clipboard_event(
        &mut self,
        event: super::clipboard::ClipboardEvent,
        connection: &super::connection::TcpConnectionProvider,
    ) {
        match event {
            super::clipboard::ClipboardEvent::ContentChanged { content, mime_type } => {
                if self.data.active_connection.is_some() {
                    info!("[continuity] clipboard changed, sending to peer");
                    connection.send_message(super::protocol::Message::ClipboardUpdate {
                        content,
                        mime_type,
                    });
                }
            }
        }
    }
}
