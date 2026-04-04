use async_channel::Sender;
use std::time::{Instant, Duration};
use log::{error, info};

use super::{
    ContinuityInner, ContinuityCmd, ContinuityData, SharingState, PeerConfig, PeerArrangement,
    PendingPin, CONTINUITY_PORT,
};
use super::clipboard::ClipboardSync;
use super::connection::{ConnectionProvider, TcpConnectionProvider};
use super::discovery::{AvahiDiscovery, DiscoveryProvider};
use super::input::{EvdevCapture, InputCapture, InputInjection, WaylandInjection};
use super::known_peers;
use super::protocol;

impl ContinuityInner {
    pub(super) async fn handle_set_enabled(
        &mut self,
        on: bool,
        discovery: &mut AvahiDiscovery,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        discovery_tx: &Sender<super::discovery::DiscoveryEvent>,
        conn_tx: &Sender<super::connection::ConnectionEvent>,
    ) {
        if self.data.enabled == on {
            return;
        }
        self.data.enabled = on;
        if on {
            info!("[continuity] enabled");
            if let Err(e) = discovery.register(&self.data.device_name, CONTINUITY_PORT) {
                error!("[continuity] discovery register failed: {e}");
            }
            if let Err(e) = discovery.browse(discovery_tx.clone()) {
                error!("[continuity] discovery browse failed: {e}");
            }
            if let Err(e) = connection.listen(CONTINUITY_PORT, conn_tx.clone()) {
                error!("[continuity] listen failed: {e}");
            }
        } else {
            info!("[continuity] disabled");
            discovery.stop();
            connection.stop();
            clipboard.stop_monitoring();
            injection.stop();
            capture.stop();
            self.data.peers.clear();
            self.data.active_connection = None;
            self.data.sharing_state = SharingState::Idle;
            self.data.pending_pin = None;
            self.last_message_at = None;
        }
        self.push();
    }

    pub(super) async fn handle_start_discovery(
        &mut self,
        discovery: &mut AvahiDiscovery,
        discovery_tx: &Sender<super::discovery::DiscoveryEvent>,
    ) {
        if self.data.enabled {
            info!("[continuity] starting peer discovery");
            if let Err(e) = discovery.browse(discovery_tx.clone()) {
                error!("[continuity] discovery browse failed: {e}");
            }
        }
    }

    pub(super) async fn handle_stop_discovery(&mut self, discovery: &mut AvahiDiscovery) {
        discovery.stop_browse();
        self.data.peers.clear();
        self.push();
    }

    pub(super) async fn handle_connect_to_peer(
        &mut self,
        peer_id: &str,
        connection: &mut TcpConnectionProvider,
        _discovery_tx: &Sender<super::discovery::DiscoveryEvent>,
        conn_tx: &Sender<super::connection::ConnectionEvent>,
    ) {
        if let Some(peer) = self.data.peers.iter().find(|p| p.device_id == peer_id) {
            let name = peer.device_name.clone();
            let addr_v4 = peer.address;
            let addr_v6 = peer.address_v6;
            let is_trusted = self.data.peer_configs
                .get(&peer.device_id)
                .map(|c| c.trusted)
                .unwrap_or(false);

            info!("[continuity] connecting to {name} (trusted: {is_trusted})");
            self.is_initiating = true;
            self.pending_peer = Some((peer.device_id.clone(), name.clone()));

            connection.connect_dual(
                addr_v4,
                addr_v6,
                conn_tx.clone(),
                self.data.device_id.clone(),
                self.data.device_name.clone(),
            );

            if is_trusted {
                let pin = format!("{:06}", rand::random_range(0..1_000_000));
                self.data.pending_pin = Some(PendingPin {
                    pin: pin.clone(),
                    peer_id: peer.device_id.clone(),
                    peer_name: name,
                    is_incoming: false,
                    created_at: Instant::now(),
                });
                connection.send_message(protocol::Message::PinRequest { pin });
                self.push();
            }
        }
    }

    pub(super) async fn handle_confirm_pin(
        &mut self,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        _capture: &mut EvdevCapture,
        clipboard_tx: &Sender<super::clipboard::ClipboardEvent>,
    ) {
        if let Some(pending) = self.data.pending_pin.take() {
            info!("[continuity] PIN confirmed locally");
            connection.send_message(protocol::Message::PinConfirm { pin: pending.pin.clone() });

            self.data.peer_configs.entry(pending.peer_id.clone()).or_default().trusted = true;
            self.persist_known_peers();

            if pending.is_incoming {
                info!("[continuity] Connection to {} is now active", pending.peer_name);
                self.data.active_connection = Some(super::ActiveConnectionInfo {
                    peer_id: pending.peer_id,
                    peer_name: pending.peer_name,
                    since: Instant::now(),
                });
                self.last_message_at = Some(Instant::now());

                connection.send_message(protocol::Message::ScreenInfo {
                    width: self.data.screen_width,
                    height: self.data.screen_height,
                });

                if let Err(e) = clipboard.start_monitoring(clipboard_tx.clone()) {
                    error!("[continuity] failed to start clipboard monitoring: {e}");
                }

                if let Err(e) = injection.start() {
                    error!("[continuity] failed to start input injection: {e}");
                }
            }
        }
        self.push();
    }

    pub(super) async fn handle_reject_pin(
        &mut self,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] PIN rejected");
        self.data.pending_pin = None;
        connection.send_message(protocol::Message::Disconnect {
            reason: "PIN rejected".to_string(),
        });
        connection.disconnect_active();
        clipboard.stop_monitoring();
        injection.stop();
        capture.stop();
        self.data.active_connection = None;
        self.data.sharing_state = SharingState::Idle;
        self.push();
    }

    pub(super) async fn handle_disconnect(
        &mut self,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] disconnecting");
        connection.disconnect_active();
        clipboard.stop_monitoring();
        injection.stop();
        capture.stop();
        self.data.active_connection = None;
        self.data.sharing_state = SharingState::Idle;
        self.last_message_at = None;
        self.push();
    }

    pub(super) async fn handle_cancel_reconnect(&mut self) {
        if self.data.reconnect.is_some() {
            info!("[continuity] reconnect cancelled");
            self.data.reconnect = None;
            self.push();
        }
    }

    pub(super) async fn handle_unpair(
        &mut self,
        peer_id: &str,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] unpairing {peer_id}");
        self.data.peer_configs.remove(peer_id);
        self.persist_known_peers();

        if self.data.active_connection.as_ref().is_some_and(|c| c.peer_id == peer_id) {
            connection.disconnect_active();
            clipboard.stop_monitoring();
            injection.stop();
            capture.stop();
            self.data.active_connection = None;
            self.data.sharing_state = SharingState::Idle;
            self.last_message_at = None;
        }
        self.push();
    }

    pub(super) async fn handle_force_local(
        &mut self,
        capture: &mut EvdevCapture,
        connection: &mut TcpConnectionProvider,
    ) {
        if !self.data.sharing_state.is_active() {
            info!("[continuity] forcing cursor back to local");
            self.data.sharing_state = SharingState::Idle;
            capture.stop();
            connection.send_message(protocol::Message::TransitionCancel);
            self.push();
            let _ = capture.prepare();
        }
    }

    pub(super) async fn handle_start_sharing(
        &mut self,
        side: super::Side,
        local_edge_pos: f64,
        connection: &mut TcpConnectionProvider,
    ) {
        if self.data.active_connection.is_some() && self.data.sharing_state == SharingState::Idle {
            if self.last_transition_at.elapsed() < Duration::from_millis(500) {
                return;
            }

            let arrangement = self.data.active_peer_config().arrangement;
            let remote_edge_pos = arrangement.local_to_remote_edge(local_edge_pos);
            info!("[continuity] initiating sharing via {:?}, local_pos={:.0} remote_pos={:.0}", side, local_edge_pos, remote_edge_pos);
            self.data.sharing_state = SharingState::Pending { entry_side: side, edge_pos: remote_edge_pos };
            self.last_transition_at = Instant::now();

            connection.send_message(protocol::Message::EdgeTransition { side, edge_pos: remote_edge_pos });
            self.push();
        }
    }

    pub(super) async fn handle_stop_sharing(
        &mut self,
        edge_pos: f64,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        if matches!(&self.data.sharing_state, SharingState::Sharing { .. } | SharingState::Pending { .. }) {
            info!("[continuity] stopping sharing");
            self.data.sharing_state = SharingState::Idle;
            capture.stop();
            connection.send_message(protocol::Message::TransitionCancel);
            self.push();
            let _ = capture.prepare();
        } else if matches!(self.data.sharing_state, SharingState::Receiving) {
            let side = self.data.active_peer_config().arrangement.side;
            info!("[continuity] requesting switch back to sharing via {:?}, edge_pos={:.0}", side, edge_pos);
            self.data.sharing_state = SharingState::PendingSwitch;
            connection.send_message(protocol::Message::SwitchTransition { side, edge_pos });
            self.push();
        }
    }

    pub(super) async fn handle_send_input(
        &mut self,
        msg: &protocol::Message,
        connection: &mut TcpConnectionProvider,
    ) {
        if matches!(self.data.sharing_state, SharingState::Sharing { .. }) {
            connection.send_message(msg.clone());
        }
    }

    pub(super) async fn handle_set_peer_arrangement(
        &mut self,
        arrangement: PeerArrangement,
        connection: &mut TcpConnectionProvider,
    ) {
        if let Some(conn) = &self.data.active_connection {
            let peer_id = conn.peer_id.clone();
            let config = self.data.peer_configs.entry(peer_id).or_default();
            config.arrangement = arrangement;
            config.version += 1;
            let version = config.version;

            info!("[continuity] updated config for peer {}: {:?} (v{})", conn.peer_name, arrangement, version);

            connection.send_message(protocol::Message::ConfigSync {
                arrangement: arrangement.side,
                offset: arrangement.offset,
                clipboard: config.clipboard,
                audio: config.audio,
                drag_drop: config.drag_drop,
                version,
            });
        }
        self.push();
    }

    pub(super) async fn handle_update_peer_configs(
        &mut self,
        configs: std::collections::HashMap<String, PeerConfig>,
    ) {
        let mut changed = false;
        for (id, config) in configs {
            let entry = self.data.peer_configs.entry(id).or_default();
            if entry.version < config.version || (entry.version == config.version && entry.arrangement != config.arrangement) {
                *entry = config;
                changed = true;
            }
        }
        if changed {
            self.push();
        }
    }

    pub(super) async fn handle_set_screen_size(&mut self, w: i32, h: i32) {
        info!("[continuity] screen size set to {}x{}", w, h);
        self.data.screen_width = w;
        self.data.screen_height = h;
        self.push();
    }

    pub(super) async fn handle_switch_to_receiving(
        &mut self,
        side: super::Side,
        connection: &mut TcpConnectionProvider,
        injection: &mut WaylandInjection,
    ) {
        let virtual_pos = match &self.data.sharing_state {
            SharingState::Sharing { virtual_pos, .. } => Some(*virtual_pos),
            SharingState::PendingSwitch => None,
            _ => None,
        };
        if virtual_pos.is_some() || matches!(self.data.sharing_state, SharingState::PendingSwitch) {
            let edge_pos = match side {
                super::Side::Left | super::Side::Right => virtual_pos.map(|v| v.1).unwrap_or(0.0),
                super::Side::Top | super::Side::Bottom => virtual_pos.map(|v| v.0).unwrap_or(0.0),
            };
            info!("[continuity] switching to Receiving via {:?}, edge_pos={:.0}", side, edge_pos);

            self.data.sharing_state = SharingState::Receiving;

            if let Err(e) = injection.start() {
                error!("[continuity] failed to start injection for switch: {e}");
            }

            if let Err(e) = injection.warp(side, edge_pos, self.data.screen_width, self.data.screen_height) {
                error!("[continuity] failed to warp cursor for switch: {e}");
            }

            connection.send_message(protocol::Message::SwitchConfirm { side, edge_pos });
            self.push();
        }
    }
}
