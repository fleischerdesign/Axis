use std::pin::Pin;
use std::time::{Duration, Instant};

use async_channel::Sender;
use axis_domain::models::continuity::{
    ActiveConnectionInfo, Message, PeerArrangement, PendingPin, SharingState, Side,
};
use log::{error, info, warn};

use super::super::clipboard::{ClipboardEvent, ClipboardSync, WaylandClipboard};
use super::super::connection::{ConnectionEvent, ConnectionProvider, TcpConnectionProvider};
use super::super::input::{
    EvdevCapture, InputCapture, InputInjection, InternalInputEvent, WaylandInjection,
};
use super::super::proto;
use super::{
    CONNECTION_TIMEOUT_SECS, CmdContext, ConfigSyncArgs, ContinuityInner, PIN_EXPIRY_SECS,
};

impl ContinuityInner {
    pub(crate) fn handle_heartbeat(
        &mut self,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        if let Some(last) = self.last_message_at {
            let timeout = if matches!(
                self.status.sharing_state,
                SharingState::Receiving
                    | SharingState::Pending { .. }
                    | SharingState::PendingSwitch
            ) {
                Duration::from_secs(5)
            } else {
                Duration::from_secs(CONNECTION_TIMEOUT_SECS)
            };

            if last.elapsed() > timeout {
                warn!("[continuity] peer timed out (no message for {:?})", timeout);
                connection.disconnect_active();
                capture.stop();
                self.status.active_connection = None;
                self.connected_at = None;
                self.status.sharing_state = SharingState::Idle;
                self.last_message_at = None;
                self.push();
                return;
            }
        }

        connection.send_message(Message::Heartbeat);

        if self.status.pending_pin.is_some() {
            let elapsed = self
                .pin_created_at
                .map(|t| t.elapsed())
                .unwrap_or(Duration::ZERO);
            if elapsed > Duration::from_secs(PIN_EXPIRY_SECS) {
                warn!("[continuity] PIN expired ({}s timeout)", PIN_EXPIRY_SECS);
                self.status.pending_pin = None;
                self.pin_created_at = None;
                connection.send_message(Message::Disconnect {
                    reason: "PIN expired".to_string(),
                });
                connection.disconnect_active();
                self.push();
            }
        }
    }

    pub(crate) async fn handle_connection_event(
        &mut self,
        event: ConnectionEvent,
        ctx: &mut CmdContext<'_>,
    ) -> Option<Pin<Box<tokio::time::Sleep>>> {
        match event {
            ConnectionEvent::IncomingConnection { addr, write_tx } => {
                self.handle_incoming_connection(addr, write_tx, ctx.connection)
                    .await;
                None
            }
            ConnectionEvent::HandshakeComplete { .. } => None,
            ConnectionEvent::Disconnected { reason } => {
                self.handle_disconnected(
                    reason,
                    ctx.connection,
                    ctx.clipboard,
                    ctx.injection,
                    ctx.capture,
                )
                .await
            }
            ConnectionEvent::MessageReceived(msg) => {
                self.handle_message_received(msg, ctx).await;
                None
            }
            ConnectionEvent::Error(e) => {
                error!("[continuity] connection error: {e}");
                None
            }
        }
    }

    pub(crate) async fn handle_incoming_connection(
        &mut self,
        addr: std::net::SocketAddr,
        write_tx: tokio::sync::mpsc::Sender<Message>,
        connection: &mut TcpConnectionProvider,
    ) {
        info!("[continuity] incoming connection from {addr}");
        self.is_initiating = false;
        connection.set_active_write(write_tx);

        let hello = Message::Hello {
            device_id: self.status.device_id.clone(),
            device_name: self.status.device_name.clone(),
            version: proto::PROTOCOL_VERSION,
        };
        connection.send_message(hello);
    }

    pub(crate) async fn handle_disconnected(
        &mut self,
        reason: String,
        _connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) -> Option<Pin<Box<tokio::time::Sleep>>> {
        info!("[continuity] disconnected: {reason}");
        let was_active = self.status.active_connection.take();
        self.connected_at = None;
        self.status.sharing_state = SharingState::Idle;
        self.status.pending_pin = None;
        self.pin_created_at = None;
        self.status.remote_screen = None;
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

    pub(crate) async fn handle_message_received(&mut self, msg: Message, ctx: &mut CmdContext<'_>) {
        self.last_message_at = Some(Instant::now());
        match msg {
            Message::Hello {
                device_id,
                device_name,
                version,
            } => {
                self.handle_hello(
                    device_id,
                    device_name,
                    version,
                    ctx.connection,
                    ctx.injection,
                    ctx.capture,
                )
                .await;
            }
            Message::PinRequest { pin } => {
                self.handle_pin_request(pin).await;
            }
            Message::PinConfirm { pin } => {
                self.handle_pin_confirm(
                    pin,
                    ctx.connection,
                    ctx.clipboard,
                    ctx.injection,
                    ctx.capture,
                    ctx.clipboard_tx,
                )
                .await;
            }
            Message::ClipboardUpdate { content, mime_type } => {
                if let Err(e) = ctx.clipboard.set_content(&content, &mime_type) {
                    error!("[continuity] failed to set clipboard: {e}");
                }
            }
            Message::DragOffer {
                transfer_id,
                file_name,
                file_size,
                mime_type,
                is_directory,
                item_count,
            } => {
                info!(
                    "[continuity] incoming file drag offer {}: {} ({} bytes)",
                    transfer_id, file_name, file_size
                );
                self.status.active_drag =
                    Some(axis_domain::models::continuity::ActiveDragPayload {
                        transfer_id: transfer_id.clone(),
                        name: file_name.clone(),
                        size_bytes: file_size,
                        mime_type: mime_type.clone(),
                        is_directory,
                        item_count,
                    });
                self.push();
                let _ = ctx
                    .drag_drop_mgr
                    .handle_offer(transfer_id, file_name, file_size, mime_type)
                    .await;
            }
            Message::DragChunk {
                transfer_id,
                chunk_index,
                is_last,
                data,
            } => {
                if let Ok(Some(completed_path)) = ctx
                    .drag_drop_mgr
                    .handle_chunk(&transfer_id, chunk_index, is_last, &data)
                    .await
                {
                    info!(
                        "[continuity] file drag transfer complete: saved to {:?}",
                        completed_path
                    );
                    self.status.active_drag = None;
                    self.push();
                    let uri = format!("file://{}", completed_path.to_string_lossy());
                    if let Err(e) = ctx.clipboard.set_content(uri.as_bytes(), "text/uri-list") {
                        error!("[continuity] failed to set clipboard for file drop: {e}");
                    }
                }
            }
            Message::DragCancel { transfer_id: _ } => {
                info!("[continuity] file drag transfer cancelled");
                self.status.active_drag = None;
                self.push();
            }
            Message::CursorMove { .. }
            | Message::KeyPress { .. }
            | Message::KeyRelease { .. }
            | Message::PointerButton { .. }
            | Message::PointerAxis { .. } => {
                let _ = ctx.injection.inject(&msg);
            }
            Message::ScreenInfo { width, height } => {
                self.handle_screen_info(width, height, ctx.connection, ctx.capture)
                    .await;
            }
            Message::ConfigSync {
                arrangement,
                offset,
                clipboard: cb,
                audio,
                drag_drop,
                version,
            } => {
                self.handle_config_sync(
                    ConfigSyncArgs {
                        arrangement,
                        offset,
                        clipboard: cb,
                        audio,
                        drag_drop,
                        version,
                    },
                    ctx,
                )
                .await;
            }
            Message::EdgeTransition { side, edge_pos } => {
                self.handle_edge_transition(side, edge_pos, ctx.connection, ctx.injection)
                    .await;
            }
            Message::TransitionAck { accepted } => {
                self.handle_transition_ack(accepted, ctx.connection, ctx.capture, ctx.input_tx)
                    .await;
            }
            Message::TransitionCancel => {
                self.handle_transition_cancel().await;
            }
            Message::SwitchTransition { side, edge_pos: _ } => {
                self.handle_switch_transition(side, ctx.connection).await;
            }
            Message::SwitchConfirm { side, edge_pos } => {
                self.handle_switch_confirm(
                    side,
                    edge_pos,
                    ctx.connection,
                    ctx.capture,
                    ctx.input_tx,
                )
                .await;
            }
            Message::Connected => {
                info!("[continuity] connection established");
            }
            Message::Heartbeat => {}
            Message::Disconnect { reason } => {
                self.handle_peer_disconnect(
                    reason,
                    ctx.connection,
                    ctx.clipboard,
                    ctx.injection,
                    ctx.capture,
                )
                .await;
            }
        }
    }

    pub(crate) async fn handle_hello(
        &mut self,
        device_id: String,
        device_name: String,
        version: u32,
        connection: &mut TcpConnectionProvider,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        if version != proto::PROTOCOL_VERSION {
            warn!("[continuity] peer version mismatch: {version}");
            connection.disconnect_active();
            self.last_message_at = None;
            return;
        }
        info!("[continuity] handshake from {device_name} ({device_id})");
        self.pending_peer = Some((device_id.clone(), device_name.clone()));

        if self.is_initiating {
            let is_trusted = self
                .status
                .peer_configs
                .get(&device_id)
                .map(|c| c.trusted)
                .unwrap_or(false);
            if is_trusted {
                info!("[continuity] trusted peer reconnected, skipping PIN");
                self.status.active_connection = Some(ActiveConnectionInfo {
                    peer_id: device_id,
                    peer_name: device_name,
                    connected_secs: 0,
                });
                self.connected_at = Some(Instant::now());
                self.status.pending_pin = None;
                self.pin_created_at = None;
                self.status.reconnect = None;
                self.last_message_at = Some(Instant::now());
                self.push();

                connection.send_message(Message::ScreenInfo {
                    width: self.status.screen_width,
                    height: self.status.screen_height,
                });

                if let Err(e) = injection.start() {
                    error!("[continuity] failed to start input injection: {e}");
                }
                let _ = capture.prepare();
            } else {
                let pin = format!("{:06}", rand::random_range(0..1_000_000));
                info!("[continuity] initiating pairing, generating PIN: {pin}");
                self.status.pending_pin = Some(PendingPin {
                    pin: pin.clone(),
                    peer_id: device_id,
                    peer_name: device_name,
                    is_incoming: false,
                });
                self.pin_created_at = Some(Instant::now());
                connection.send_message(Message::PinRequest { pin });
                self.push();
            }
        } else {
            let is_trusted = self
                .status
                .peer_configs
                .get(&device_id)
                .map(|c| c.trusted)
                .unwrap_or(false);
            if is_trusted {
                info!("[continuity] trusted peer connected (incoming), skipping PIN");
                self.status.active_connection = Some(ActiveConnectionInfo {
                    peer_id: device_id,
                    peer_name: device_name,
                    connected_secs: 0,
                });
                self.connected_at = Some(Instant::now());
                self.status.pending_pin = None;
                self.pin_created_at = None;
                self.status.reconnect = None;
                self.last_message_at = Some(Instant::now());
                self.push();

                connection.send_message(Message::ScreenInfo {
                    width: self.status.screen_width,
                    height: self.status.screen_height,
                });

                if let Err(e) = injection.start() {
                    error!("[continuity] failed to start input injection: {e}");
                }
                let _ = capture.prepare();
            } else {
                let pin = format!("{:06}", rand::random_range(0..1_000_000));
                info!("[continuity] incoming pairing request, generating PIN: {pin}");
                self.status.pending_pin = Some(PendingPin {
                    pin: pin.clone(),
                    peer_id: device_id,
                    peer_name: device_name,
                    is_incoming: true,
                });
                self.pin_created_at = Some(Instant::now());
                connection.send_message(Message::PinRequest { pin });
                self.push();
            }
        }
    }

    pub(crate) async fn handle_pin_request(&mut self, pin: String) {
        if let Some((peer_id, peer_name)) = self.pending_peer.clone() {
            info!("[continuity] received pairing request with PIN: {pin}");
            self.status.pending_pin = Some(PendingPin {
                pin,
                peer_id,
                peer_name,
                is_incoming: true,
            });
            self.pin_created_at = Some(Instant::now());
            self.push();
        }
    }

    pub(crate) async fn handle_pin_confirm(
        &mut self,
        pin: String,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        clipboard_tx: &Sender<ClipboardEvent>,
    ) {
        if let Some(pending) = &self.status.pending_pin {
            if pending.pin == pin {
                info!("[continuity] peer confirmed PIN, connection active");

                self.status
                    .peer_configs
                    .entry(pending.peer_id.clone())
                    .or_default()
                    .trusted = true;
                self.persist_known_peers();

                self.status.active_connection = Some(ActiveConnectionInfo {
                    peer_id: pending.peer_id.clone(),
                    peer_name: pending.peer_name.clone(),
                    connected_secs: 0,
                });
                self.connected_at = Some(Instant::now());
                self.status.pending_pin = None;
                self.pin_created_at = None;
                self.push();

                connection.send_message(Message::ScreenInfo {
                    width: self.status.screen_width,
                    height: self.status.screen_height,
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

    pub(crate) async fn handle_screen_info(
        &mut self,
        width: i32,
        height: i32,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] peer screen: {}x{}", width, height);
        self.status.remote_screen = Some((width, height));
        self.push();

        let config = self.status.active_peer_config();
        connection.send_message(Message::ConfigSync {
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

    pub(crate) async fn handle_config_sync(
        &mut self,
        args: ConfigSyncArgs,
        _ctx: &mut CmdContext<'_>,
    ) {
        if let Some(conn) = &self.status.active_connection {
            let peer_id = conn.peer_id.clone();
            let config = self.status.peer_configs.entry(peer_id).or_default();

            if args.version > config.version {
                info!(
                    "[continuity] adopting newer config from peer (v{} > v{}): {:?} offset {} clipboard={} audio={} dnd={}",
                    args.version,
                    config.version,
                    args.arrangement,
                    args.offset,
                    args.clipboard,
                    args.audio,
                    args.drag_drop
                );

                config.arrangement = PeerArrangement {
                    side: args.arrangement.opposite(),
                    offset: -args.offset,
                };
                config.clipboard = args.clipboard;
                config.audio = args.audio;
                config.drag_drop = args.drag_drop;
                config.version = args.version;
                self.push();
            } else {
                info!(
                    "[continuity] ignoring older/same config from peer (v{} <= v{})",
                    args.version, config.version
                );
            }
        }
    }

    pub(crate) async fn handle_edge_transition(
        &mut self,
        side: Side,
        edge_pos: f64,
        connection: &mut TcpConnectionProvider,
        injection: &mut WaylandInjection,
    ) {
        if self.status.sharing_state.is_idle() {
            let mapped_pos = edge_pos;
            let local_side = side.opposite();

            info!(
                "[continuity] accepting sharing from peer: peer_exit={:?}@{} -> local_entry={:?}@{}",
                side, edge_pos, local_side, mapped_pos
            );

            self.status.sharing_state = SharingState::Receiving;

            if let Err(e) = injection.warp(
                local_side,
                mapped_pos,
                self.status.screen_width,
                self.status.screen_height,
            ) {
                error!("[continuity] failed to warp cursor: {e}");
            }

            connection.send_message(Message::TransitionAck { accepted: true });
            self.push();
        } else {
            info!(
                "[continuity] rejecting sharing from peer (state: {:?})",
                self.status.sharing_state
            );
            connection.send_message(Message::TransitionAck { accepted: false });
        }
    }

    pub(crate) async fn handle_transition_ack(
        &mut self,
        accepted: bool,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
        input_tx: &Sender<InternalInputEvent>,
    ) {
        if let SharingState::Pending {
            entry_side,
            edge_pos,
        } = self.status.sharing_state.clone()
        {
            if accepted {
                info!(
                    "[continuity] transition accepted, sharing via {:?}, edge_pos={:.0}",
                    entry_side, edge_pos
                );
                let (rw, rh) = self.remote_screen();
                let virtual_pos = Self::init_virtual_pos(entry_side, edge_pos, rw, rh);
                info!(
                    "[continuity] virtual_pos initialized to ({:.0}, {:.0})",
                    virtual_pos.0, virtual_pos.1
                );

                if let Err(e) = capture.start(input_tx.clone()) {
                    error!("[continuity] failed to start input capture: {e}");
                    self.status.sharing_state = SharingState::Idle;
                    connection.send_message(Message::TransitionCancel);
                } else {
                    self.status.sharing_state = SharingState::Sharing {
                        entry_side,
                        virtual_pos,
                    };
                }
            } else {
                info!("[continuity] transition rejected by peer");
                self.status.sharing_state = SharingState::Idle;
            }
            self.push();
        }
    }

    pub(crate) async fn handle_transition_cancel(&mut self) {
        info!("[continuity] forcing cursor back to local");
        self.status.sharing_state = SharingState::Idle;
        self.push();
    }

    pub(crate) async fn handle_switch_transition(
        &mut self,
        side: Side,
        connection: &mut TcpConnectionProvider,
    ) {
        if matches!(self.status.sharing_state, SharingState::Sharing { .. }) {
            info!("[continuity] peer requesting switch via {:?}", side);
            self.status.sharing_state = SharingState::PendingSwitch;
            self.push();
            if let Some(tx) = &self.switch_tx {
                let _ = tx.try_send(super::ContinuityCmd::SwitchToReceiving(side));
            }
        } else {
            info!(
                "[continuity] rejecting switch (not in Sharing, currently {:?})",
                self.status.sharing_state
            );
            connection.send_message(Message::SwitchConfirm {
                side,
                edge_pos: 0.0,
            });
        }
    }

    pub(crate) async fn handle_switch_confirm(
        &mut self,
        side: Side,
        edge_pos: f64,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
        input_tx: &Sender<InternalInputEvent>,
    ) {
        if matches!(self.status.sharing_state, SharingState::PendingSwitch) {
            info!(
                "[continuity] switch confirmed, taking over as Sharer via {:?}, edge_pos={:.0}",
                side, edge_pos
            );

            let (rw, rh) = self.remote_screen();
            let virtual_pos = Self::init_virtual_pos(side, edge_pos.max(0.0), rw, rh);
            info!(
                "[continuity] virtual_pos initialized to ({:.0}, {:.0})",
                virtual_pos.0, virtual_pos.1
            );

            if let Err(e) = capture.start(input_tx.clone()) {
                error!("[continuity] failed to start input capture after switch: {e}");
                self.status.sharing_state = SharingState::Idle;
                connection.send_message(Message::TransitionCancel);
            } else {
                self.status.sharing_state = SharingState::Sharing {
                    entry_side: side,
                    virtual_pos,
                };
            }
            self.push();
        }
    }

    pub(crate) async fn handle_peer_disconnect(
        &mut self,
        reason: String,
        _connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] peer disconnected: {reason}");
        self.status.active_connection = None;
        self.connected_at = None;
        self.status.sharing_state = SharingState::Idle;
        self.status.pending_pin = None;
        self.pin_created_at = None;
        self.pending_peer = None;
        self.last_message_at = None;
        clipboard.stop_monitoring();
        injection.stop();
        capture.stop();
        self.push();
    }
}
