use async_channel::Sender;
use std::time::Instant;
use log::{error, info, warn};

use super::{
    ContinuityInner, SharingState, ActiveConnectionInfo, PendingPin,
};
use super::clipboard::ClipboardSync;
use super::connection::{ConnectionEvent, ConnectionProvider, TcpConnectionProvider};
use super::input::{EvdevCapture, InputCapture, InputInjection, WaylandInjection};
use super::protocol;
use super::reconnect::ReconnectSleep;

impl ContinuityInner {
    pub(super) async fn handle_connection_event(
        &mut self,
        event: ConnectionEvent,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        clipboard_tx: &Sender<super::clipboard::ClipboardEvent>,
        input_tx: &Sender<super::input::InputEvent>,
    ) -> Option<ReconnectSleep> {
        match event {
            ConnectionEvent::IncomingConnection { addr, write_tx } => {
                self.handle_incoming_connection(addr, write_tx, connection).await;
                None
            }
            ConnectionEvent::HandshakeComplete { .. } => None,
            ConnectionEvent::Disconnected { reason } => {
                self.handle_disconnected(reason, connection, clipboard, injection, capture).await
            }
            ConnectionEvent::MessageReceived(msg) => {
                self.handle_message_received(msg, connection, clipboard, injection, capture, clipboard_tx, input_tx).await;
                None
            }
            ConnectionEvent::Error(e) => {
                error!("[continuity] connection error: {e}");
                None
            }
        }
    }

    pub(super) async fn handle_incoming_connection(
        &mut self,
        addr: std::net::SocketAddr,
        write_tx: tokio::sync::mpsc::Sender<protocol::Message>,
        connection: &mut TcpConnectionProvider,
    ) {
        info!("[continuity] incoming connection from {addr}");
        self.is_initiating = false;
        connection.set_active_write(write_tx);

        let hello = protocol::Message::Hello {
            device_id: self.data.device_id.clone(),
            device_name: self.data.device_name.clone(),
            version: protocol::PROTOCOL_VERSION,
        };
        connection.send_message(hello);
    }

    pub(super) async fn handle_disconnected(
        &mut self,
        reason: String,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) -> Option<ReconnectSleep> {
        info!("[continuity] disconnected: {reason}");
        let was_active = self.data.active_connection.take();
        self.data.sharing_state = SharingState::Idle;
        self.data.pending_pin = None;
        self.data.remote_screen = None;
        self.pending_peer = None;
        self.last_message_at = None;
        clipboard.stop_monitoring();
        injection.stop();
        capture.stop();

        let reconnect_sleep = if let Some(conn) = was_active {
            self.start_reconnect(&conn.peer_id, &conn.peer_name)
        } else {
            None
        };
        self.push();
        reconnect_sleep
    }

    pub(super) async fn handle_message_received(
        &mut self,
        msg: protocol::Message,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        clipboard_tx: &Sender<super::clipboard::ClipboardEvent>,
        input_tx: &Sender<super::input::InputEvent>,
    ) {
        self.last_message_at = Some(Instant::now());
        match msg {
            protocol::Message::Hello { device_id, device_name, version } => {
                self.handle_hello(device_id, device_name, version, connection, injection, capture).await;
            }
            protocol::Message::PinRequest { pin } => {
                self.handle_pin_request(pin).await;
            }
            protocol::Message::PinConfirm { pin } => {
                self.handle_pin_confirm(pin, connection, clipboard, injection, capture, clipboard_tx).await;
            }
            protocol::Message::ClipboardUpdate { content, mime_type } => {
                if let Err(e) = clipboard.set_content(&content, &mime_type) {
                    error!("[continuity] failed to set clipboard: {e}");
                }
            }
            protocol::Message::CursorMove { .. }
            | protocol::Message::KeyPress { .. }
            | protocol::Message::KeyRelease { .. }
            | protocol::Message::PointerButton { .. }
            | protocol::Message::PointerAxis { .. } => {
                let _ = injection.inject(&msg);
            }
            protocol::Message::ScreenInfo { width, height } => {
                self.handle_screen_info(width, height, connection, capture).await;
            }
            protocol::Message::ConfigSync { arrangement, offset, clipboard: cb, audio, drag_drop, version } => {
                self.handle_config_sync(arrangement, offset, cb, audio, drag_drop, version, connection).await;
            }
            protocol::Message::EdgeTransition { side, edge_pos } => {
                self.handle_edge_transition(side, edge_pos, connection, injection).await;
            }
            protocol::Message::TransitionAck { accepted } => {
                self.handle_transition_ack(accepted, connection, capture, input_tx).await;
            }
            protocol::Message::TransitionCancel => {
                self.handle_transition_cancel().await;
            }
            protocol::Message::SwitchTransition { side, edge_pos: _ } => {
                self.handle_switch_transition(side, connection).await;
            }
            protocol::Message::SwitchConfirm { side, edge_pos } => {
                self.handle_switch_confirm(side, edge_pos, connection, capture, input_tx).await;
            }
            protocol::Message::Connected => {
                info!("[continuity] connection established");
            }
            protocol::Message::Heartbeat => {}
            protocol::Message::Disconnect { reason } => {
                self.handle_peer_disconnect(reason, connection, clipboard, injection, capture).await;
            }
        }
    }

    pub(super) async fn handle_hello(
        &mut self,
        device_id: String,
        device_name: String,
        version: u32,
        connection: &mut TcpConnectionProvider,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        if version != protocol::PROTOCOL_VERSION {
            warn!("[continuity] peer version mismatch: {version}");
            connection.disconnect_active();
            self.last_message_at = None;
            return;
        }
        info!("[continuity] handshake from {device_name} ({device_id})");
        self.pending_peer = Some((device_id.clone(), device_name.clone()));

        if self.is_initiating {
            let is_trusted = self.data.peer_configs.get(&device_id).map(|c| c.trusted).unwrap_or(false);
            if is_trusted {
                info!("[continuity] trusted peer reconnected, skipping PIN");
                self.data.active_connection = Some(ActiveConnectionInfo {
                    peer_id: device_id,
                    peer_name: device_name,
                    since: Instant::now(),
                });
                self.data.pending_pin = None;
                self.data.reconnect = None;
                self.last_message_at = Some(Instant::now());
                self.push();

                connection.send_message(protocol::Message::ScreenInfo {
                    width: self.data.screen_width,
                    height: self.data.screen_height,
                });

                if let Err(e) = injection.start() {
                    error!("[continuity] failed to start input injection: {e}");
                }
                let _ = capture.prepare();
            } else {
                let pin = format!("{:06}", rand::random_range(0..1_000_000));
                info!("[continuity] initiating pairing, generating PIN: {pin}");
                self.data.pending_pin = Some(PendingPin {
                    pin: pin.clone(),
                    peer_id: device_id,
                    peer_name: device_name,
                    is_incoming: false,
                    created_at: Instant::now(),
                });
                connection.send_message(protocol::Message::PinRequest { pin });
                self.push();
            }
        } else {
            let is_trusted = self.data.peer_configs.get(&device_id).map(|c| c.trusted).unwrap_or(false);
            if is_trusted {
                info!("[continuity] trusted peer connected (incoming), skipping PIN");
                self.data.active_connection = Some(ActiveConnectionInfo {
                    peer_id: device_id,
                    peer_name: device_name,
                    since: Instant::now(),
                });
                self.data.pending_pin = None;
                self.data.reconnect = None;
                self.last_message_at = Some(Instant::now());
                self.push();

                connection.send_message(protocol::Message::ScreenInfo {
                    width: self.data.screen_width,
                    height: self.data.screen_height,
                });

                if let Err(e) = injection.start() {
                    error!("[continuity] failed to start input injection: {e}");
                }
                let _ = capture.prepare();
            } else {
                let pin = format!("{:06}", rand::random_range(0..1_000_000));
                info!("[continuity] incoming pairing request, generating PIN: {pin}");
                self.data.pending_pin = Some(PendingPin {
                    pin: pin.clone(),
                    peer_id: device_id,
                    peer_name: device_name,
                    is_incoming: true,
                    created_at: Instant::now(),
                });
                connection.send_message(protocol::Message::PinRequest { pin });
                self.push();
            }
        }
    }

    pub(super) async fn handle_pin_request(&mut self, pin: String) {
        if let Some((peer_id, peer_name)) = self.pending_peer.clone() {
            info!("[continuity] received pairing request with PIN: {pin}");
            self.data.pending_pin = Some(PendingPin {
                pin,
                peer_id,
                peer_name,
                is_incoming: true,
                created_at: Instant::now(),
            });
            self.push();
        }
    }

    pub(super) async fn handle_pin_confirm(
        &mut self,
        pin: String,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        clipboard_tx: &Sender<super::clipboard::ClipboardEvent>,
    ) {
        if let Some(pending) = &self.data.pending_pin {
            if pending.pin == pin {
                info!("[continuity] peer confirmed PIN, connection active");

                self.data.peer_configs.entry(pending.peer_id.clone()).or_default().trusted = true;
                self.persist_known_peers();

                self.data.active_connection = Some(ActiveConnectionInfo {
                    peer_id: pending.peer_id.clone(),
                    peer_name: pending.peer_name.clone(),
                    since: Instant::now(),
                });
                self.data.pending_pin = None;
                self.push();

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

                self.push();
                let _ = capture.prepare();
            } else {
                warn!("[continuity] peer sent incorrect PIN confirmation");
                connection.disconnect_active();
            }
        }
    }

    pub(super) async fn handle_screen_info(
        &mut self,
        width: i32,
        height: i32,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] peer screen: {}x{}", width, height);
        self.data.remote_screen = Some((width, height));
        self.push();

        let config = self.data.active_peer_config();
        connection.send_message(protocol::Message::ConfigSync {
            arrangement: config.arrangement.side,
            offset: config.arrangement.offset,
            clipboard: config.clipboard,
            audio: config.audio,
            drag_drop: config.drag_drop,
            version: config.version,
        });

        self.push();
        let _ = capture.prepare();
    }

    pub(super) async fn handle_config_sync(
        &mut self,
        arrangement: super::Side,
        offset: i32,
        clipboard: bool,
        audio: bool,
        drag_drop: bool,
        version: u64,
        connection: &mut TcpConnectionProvider,
    ) {
        if let Some(conn) = &self.data.active_connection {
            let peer_id = conn.peer_id.clone();
            let config = self.data.peer_configs.entry(peer_id).or_default();

            if version > config.version {
                info!("[continuity] adopting newer config from peer (v{} > v{}): {:?} offset {} clipboard={} audio={} dnd={}",
                    version, config.version, arrangement, offset, clipboard, audio, drag_drop);

                config.arrangement = super::PeerArrangement {
                    side: arrangement.opposite(),
                    offset: -offset,
                };
                config.clipboard = clipboard;
                config.audio = audio;
                config.drag_drop = drag_drop;
                config.version = version;
                self.push();
            } else {
                info!("[continuity] ignoring older/same config from peer (v{} <= v{})", version, config.version);
            }
        }
    }

    pub(super) async fn handle_edge_transition(
        &mut self,
        side: super::Side,
        edge_pos: f64,
        connection: &mut TcpConnectionProvider,
        injection: &mut WaylandInjection,
    ) {
        if self.data.sharing_state.is_idle() {
            let mapped_pos = edge_pos;
            let local_side = side.opposite();

            info!("[continuity] accepting sharing from peer: peer_exit={:?}@{} -> local_entry={:?}@{}",
                side, edge_pos, local_side, mapped_pos);

            self.data.sharing_state = SharingState::Receiving;

            if let Err(e) = injection.warp(local_side, mapped_pos, self.data.screen_width, self.data.screen_height) {
                error!("[continuity] failed to warp cursor: {e}");
            }

            connection.send_message(protocol::Message::TransitionAck { accepted: true });
            self.push();
        } else {
            info!("[continuity] rejecting sharing from peer (state: {:?})", self.data.sharing_state);
            connection.send_message(protocol::Message::TransitionAck { accepted: false });
        }
    }

    pub(super) async fn handle_transition_ack(
        &mut self,
        accepted: bool,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
        input_tx: &Sender<super::input::InputEvent>,
    ) {
        if let SharingState::Pending { entry_side, edge_pos } = self.data.sharing_state.clone() {
            if accepted {
                info!("[continuity] transition accepted, sharing via {:?}, edge_pos={:.0}", entry_side, edge_pos);
                let (rw, rh) = self.remote_screen();
                let virtual_pos = Self::init_virtual_pos(entry_side, edge_pos, rw, rh);
                info!("[continuity] virtual_pos initialized to ({:.0}, {:.0})", virtual_pos.0, virtual_pos.1);

                if let Err(e) = capture.start(input_tx.clone()) {
                    error!("[continuity] failed to start input capture: {e}");
                    self.data.sharing_state = SharingState::Idle;
                    connection.send_message(protocol::Message::TransitionCancel);
                } else {
                    self.data.sharing_state = SharingState::Sharing { entry_side, virtual_pos };
                }
            } else {
                info!("[continuity] transition rejected by peer");
                self.data.sharing_state = SharingState::Idle;
            }
            self.push();
        }
    }

    pub(super) async fn handle_transition_cancel(&mut self) {
        info!("[continuity] forcing cursor back to local");
        self.data.sharing_state = SharingState::Idle;
        self.push();
    }

    pub(super) async fn handle_switch_transition(
        &mut self,
        side: super::Side,
        connection: &mut TcpConnectionProvider,
    ) {
        if matches!(self.data.sharing_state, SharingState::Sharing { .. }) {
            info!("[continuity] peer requesting switch via {:?}", side);
            self.data.sharing_state = SharingState::PendingSwitch;
            self.push();
            if let Some(tx) = &self.switch_tx {
                let _ = tx.try_send(super::ContinuityCmd::SwitchToReceiving(side));
            }
        } else {
            info!("[continuity] rejecting switch (not in Sharing, currently {:?})", self.data.sharing_state);
            connection.send_message(protocol::Message::SwitchConfirm { side, edge_pos: 0.0 });
        }
    }

    pub(super) async fn handle_switch_confirm(
        &mut self,
        side: super::Side,
        edge_pos: f64,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
        input_tx: &Sender<super::input::InputEvent>,
    ) {
        if matches!(self.data.sharing_state, SharingState::PendingSwitch) {
            info!("[continuity] switch confirmed, taking over as Sharer via {:?}, edge_pos={:.0}", side, edge_pos);

            let (rw, rh) = self.remote_screen();
            let virtual_pos = Self::init_virtual_pos(side, edge_pos.max(0.0), rw, rh);
            info!("[continuity] virtual_pos initialized to ({:.0}, {:.0})", virtual_pos.0, virtual_pos.1);

            if let Err(e) = capture.start(input_tx.clone()) {
                error!("[continuity] failed to start input capture after switch: {e}");
                self.data.sharing_state = SharingState::Idle;
                connection.send_message(protocol::Message::TransitionCancel);
            } else {
                self.data.sharing_state = SharingState::Sharing { entry_side: side, virtual_pos };
            }
            self.push();
        }
    }

    pub(super) async fn handle_peer_disconnect(
        &mut self,
        reason: String,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut super::clipboard::WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] peer disconnected: {reason}");
        self.data.active_connection = None;
        self.data.sharing_state = SharingState::Idle;
        self.data.pending_pin = None;
        self.pending_peer = None;
        self.last_message_at = None;
        clipboard.stop_monitoring();
        injection.stop();
        capture.stop();
        self.push();
    }
}
