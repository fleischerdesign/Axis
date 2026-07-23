use std::collections::HashMap;
use std::time::Instant;

use async_channel::Sender;
use axis_domain::models::continuity::{
    ActiveConnectionInfo, InputEvent, Message, PeerArrangement, PeerConfig, Side,
};
use log::{error, info};

use super::super::clipboard::ClipboardEvent;
use super::super::connection::ConnectionEvent;
use super::super::discovery::DiscoveryEvent;
use super::super::known_peers::{self, KnownPeer, KnownPeerArrangementSide};
use super::super::ports::{
    ContinuityAudioPort, ContinuityCapturePort, ContinuityClipboardPort,
    ContinuityInjectionPort, ContinuityNetworkPort,
};
use super::{CONTINUITY_PORT, CmdContext, ContinuityCmd, ContinuityInner};

impl ContinuityInner {
    pub(crate) async fn handle_cmd(&mut self, cmd: ContinuityCmd, ctx: &mut CmdContext<'_>) {
        match cmd {
            ContinuityCmd::SetEnabled(on) => {
                self.handle_set_enabled(on, ctx).await;
            }
            ContinuityCmd::ConnectToPeer(peer_id) => {
                self.handle_connect_to_peer(
                    &peer_id,
                    ctx.network,
                    ctx.discovery_tx,
                    ctx.conn_tx,
                )
                .await;
            }
            ContinuityCmd::ConfirmPin => {
                self.handle_confirm_pin(
                    ctx.network,
                    ctx.clipboard,
                    ctx.injection,
                    ctx.capture,
                    ctx.clipboard_tx,
                )
                .await;
            }
            ContinuityCmd::RejectPin => {
                self.handle_reject_pin(ctx.network, ctx.clipboard, ctx.injection, ctx.capture)
                    .await;
            }
            ContinuityCmd::Disconnect => {
                self.handle_disconnect(ctx.network, ctx.clipboard, ctx.injection, ctx.capture)
                    .await;
            }
            ContinuityCmd::CancelReconnect => {
                self.handle_cancel_reconnect().await;
            }
            ContinuityCmd::Unpair(peer_id) => {
                self.handle_unpair(
                    &peer_id,
                    ctx.network,
                    ctx.clipboard,
                    ctx.injection,
                    ctx.capture,
                )
                .await;
            }
            ContinuityCmd::ForceLocal => {
                self.handle_force_local(ctx.capture, ctx.network).await;
            }
            ContinuityCmd::StartSharing(side, local_edge_pos) => {
                self.handle_start_sharing(side, local_edge_pos, ctx.network)
                    .await;
            }
            ContinuityCmd::StopSharing(edge_pos) => {
                self.handle_stop_sharing(edge_pos, ctx.network, ctx.capture)
                    .await;
            }
            ContinuityCmd::SendInput(event) => {
                self.handle_send_input(&event, ctx.network, ctx.capture)
                    .await;
            }
            ContinuityCmd::SetPeerArrangement(arrangement) => {
                self.handle_set_peer_arrangement(arrangement, ctx.network)
                    .await;
            }
            ContinuityCmd::UpdatePeerConfigs(configs) => {
                self.handle_update_peer_configs(
                    configs,
                    ctx.network,
                    ctx.clipboard,
                    ctx.clipboard_tx,
                    ctx.audio,
                )
                .await;
            }
            ContinuityCmd::SwitchToReceiving(side) => {
                self.handle_switch_to_receiving(side, ctx.network, ctx.injection)
                    .await;
            }
        }
    }

    async fn handle_set_enabled(&mut self, on: bool, ctx: &mut CmdContext<'_>) {
        if self.status.enabled == on {
            return;
        }
        self.status.enabled = on;
        if on {
            info!("[continuity] enabled");
            if let Err(e) = ctx
                .discovery
                .register(&self.status.device_name, CONTINUITY_PORT)
            {
                error!("[continuity] discovery register failed: {e}");
            }
            if let Err(e) = ctx.discovery.browse(ctx.discovery_tx.clone()) {
                error!("[continuity] discovery browse failed: {e}");
            }
            if let Err(e) = ctx.network.listen(CONTINUITY_PORT, ctx.conn_tx.clone()) {
                error!("[continuity] listen failed: {e}");
            }
        } else {
            info!("[continuity] disabled");
            ctx.discovery.stop();
            ctx.network.stop();
            ctx.clipboard.stop_monitoring();
            ctx.injection.stop_injection();
            ctx.capture.stop_capture();
            self.status.peers.clear();
            self.status.active_connection = None;
            self.connected_at = None;
            self.status.sharing_state = axis_domain::models::continuity::SharingState::Idle;
            self.status.pending_pin = None;
            self.pin_created_at = None;
            self.last_message_at = None;
        }
        self.push();
    }

    async fn handle_connect_to_peer(
        &mut self,
        peer_id: &str,
        connection: &mut dyn ContinuityNetworkPort,
        _discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
    ) {
        if let Some(peer) = self.status.peers.iter().find(|p| p.device_id == peer_id) {
            let name = peer.device_name.clone();
            let addr_v4 = peer.address;
            let addr_v6 = peer.address_v6;

            info!("[continuity] connecting to {name}");
            self.is_initiating = true;
            self.pending_peer = Some((peer.device_id.clone(), name.clone()));
            self.status.connecting_peer_id = Some(peer.device_id.clone());
            self.push();

            connection.connect_dual(
                addr_v4,
                addr_v6,
                conn_tx.clone(),
                self.status.device_id.clone(),
                self.status.device_name.clone(),
            );
        }
    }

    async fn handle_confirm_pin(
        &mut self,
        connection: &mut dyn ContinuityNetworkPort,
        clipboard: &mut dyn ContinuityClipboardPort,
        injection: &mut dyn ContinuityInjectionPort,
        _capture: &mut dyn ContinuityCapturePort,
        clipboard_tx: &Sender<ClipboardEvent>,
    ) {
        if let Some(pending) = self.status.pending_pin.take() {
            self.pin_created_at = None;
            info!("[continuity] PIN confirmed locally");
            connection.send_message(Message::PinConfirm {
                pin: pending.pin.clone(),
            });

            let config = self
                .status
                .peer_configs
                .entry(pending.peer_id.clone())
                .or_default();
            config.trusted = true;

            if let Some(known) = self.known_peers.peers.get_mut(&pending.peer_id) {
                known.trusted = true;
            } else {
                self.known_peers.peers.insert(
                    pending.peer_id.clone(),
                    KnownPeer {
                        device_id: pending.peer_id.clone(),
                        device_name: pending.peer_name.clone(),
                        hostname: String::new(),
                        address: String::new(),
                        address_v6: None,
                        trusted: true,
                        ..Default::default()
                    },
                );
            }
            known_peers::save_known_peers(&self.known_peers);

            if pending.is_incoming {
                info!(
                    "[continuity] Connection to {} is now active",
                    pending.peer_name
                );
                self.status.active_connection = Some(ActiveConnectionInfo {
                    peer_id: pending.peer_id,
                    peer_name: pending.peer_name,
                    connected_secs: 0,
                });
                self.connected_at = Some(Instant::now());
                self.last_message_at = Some(Instant::now());

                connection.send_message(Message::ScreenInfo {
                    width: self.status.screen_width,
                    height: self.status.screen_height,
                });

                if let Err(e) = clipboard.start_monitoring(clipboard_tx.clone()) {
                    error!("[continuity] failed to start clipboard monitoring: {e}");
                }

                if let Err(e) = injection.start_injection() {
                    error!("[continuity] failed to start input injection: {e}");
                }
            }
        }
        self.push();
    }

    async fn handle_reject_pin(
        &mut self,
        connection: &mut dyn ContinuityNetworkPort,
        clipboard: &mut dyn ContinuityClipboardPort,
        injection: &mut dyn ContinuityInjectionPort,
        capture: &mut dyn ContinuityCapturePort,
    ) {
        info!("[continuity] PIN rejected");
        self.status.pending_pin = None;
        self.pin_created_at = None;
        connection.send_message(Message::Disconnect {
            reason: "PIN rejected".to_string(),
        });
        connection.disconnect_active();
        clipboard.stop_monitoring();
        injection.stop_injection();
        capture.stop_capture();
        self.status.active_connection = None;
        self.connected_at = None;
        self.status.sharing_state = axis_domain::models::continuity::SharingState::Idle;
        self.push();
    }

    async fn handle_disconnect(
        &mut self,
        connection: &mut dyn ContinuityNetworkPort,
        clipboard: &mut dyn ContinuityClipboardPort,
        injection: &mut dyn ContinuityInjectionPort,
        capture: &mut dyn ContinuityCapturePort,
    ) {
        info!("[continuity] disconnecting");
        connection.disconnect_active();
        clipboard.stop_monitoring();
        injection.stop_injection();
        capture.stop_capture();
        self.status.active_connection = None;
        self.connected_at = None;
        self.status.sharing_state = axis_domain::models::continuity::SharingState::Idle;
        self.last_message_at = None;
        self.push();
    }

    async fn handle_cancel_reconnect(&mut self) {
        if self.status.reconnect.is_some() {
            info!("[continuity] reconnect cancelled");
            self.status.reconnect = None;
            self.push();
        }
    }

    async fn handle_unpair(
        &mut self,
        peer_id: &str,
        connection: &mut dyn ContinuityNetworkPort,
        clipboard: &mut dyn ContinuityClipboardPort,
        injection: &mut dyn ContinuityInjectionPort,
        capture: &mut dyn ContinuityCapturePort,
    ) {
        info!("[continuity] unpairing {peer_id}");
        self.status.peer_configs.remove(peer_id);
        self.known_peers.peers.remove(peer_id);
        known_peers::save_known_peers(&self.known_peers);

        if self
            .status
            .active_connection
            .as_ref()
            .is_some_and(|c| c.peer_id == peer_id)
        {
            connection.disconnect_active();
            clipboard.stop_monitoring();
            injection.stop_injection();
            capture.stop_capture();
            self.status.active_connection = None;
            self.connected_at = None;
            self.status.sharing_state = axis_domain::models::continuity::SharingState::Idle;
            self.last_message_at = None;
        }
        self.push();
    }

    async fn handle_force_local(
        &mut self,
        capture: &mut dyn ContinuityCapturePort,
        connection: &mut dyn ContinuityNetworkPort,
    ) {
        if !self.status.sharing_state.is_active() {
            info!("[continuity] forcing cursor back to local");
            self.status.sharing_state = axis_domain::models::continuity::SharingState::Idle;
            capture.stop_capture();
            connection.send_message(Message::TransitionCancel);
            self.push();
            let _ = capture.prepare();
        }
    }

    async fn handle_start_sharing(
        &mut self,
        side: Side,
        local_edge_pos: f64,
        connection: &mut dyn ContinuityNetworkPort,
    ) {
        if self.status.active_connection.is_some()
            && self.status.sharing_state == axis_domain::models::continuity::SharingState::Idle
        {
            if self.last_transition_at.elapsed() < std::time::Duration::from_millis(500) {
                return;
            }

            if let Ok(mut sock) = niri_ipc::socket::Socket::connect()
                && let Ok(Ok(niri_ipc::Response::FocusedWindow(Some(win)))) =
                    sock.send(niri_ipc::Request::FocusedWindow)
            {
                let is_fullscreen = win.layout.window_size.0 >= self.status.screen_width
                    && win.layout.window_size.1 >= self.status.screen_height;
                if is_fullscreen {
                    info!(
                        "[continuity] blocking edge transition: active window '{}' is in fullscreen mode",
                        win.title.unwrap_or_default()
                    );
                    return;
                }
            }

            let arrangement = self.status.active_peer_config().arrangement;
            let remote_edge_pos = arrangement.local_to_remote_edge(local_edge_pos);
            info!(
                "[continuity] initiating sharing via {:?}, local_pos={:.0} remote_pos={:.0}",
                side, local_edge_pos, remote_edge_pos
            );
            self.status.sharing_state = axis_domain::models::continuity::SharingState::Pending {
                entry_side: side,
                edge_pos: remote_edge_pos,
            };
            self.last_transition_at = Instant::now();

            connection.send_message(Message::EdgeTransition {
                side,
                edge_pos: remote_edge_pos,
            });
            self.push();
        }
    }

    async fn handle_stop_sharing(
        &mut self,
        edge_pos: f64,
        connection: &mut dyn ContinuityNetworkPort,
        capture: &mut dyn ContinuityCapturePort,
    ) {
        if matches!(
            &self.status.sharing_state,
            axis_domain::models::continuity::SharingState::Sharing { .. }
                | axis_domain::models::continuity::SharingState::Pending { .. }
        ) {
            info!("[continuity] stopping sharing");
            self.status.sharing_state = axis_domain::models::continuity::SharingState::Idle;
            capture.stop_capture();
            connection.send_message(Message::TransitionCancel);
            self.push();
            let _ = capture.prepare();
        } else if matches!(
            self.status.sharing_state,
            axis_domain::models::continuity::SharingState::Receiving
        ) {
            let side = self.status.active_peer_config().arrangement.side;
            info!(
                "[continuity] requesting switch back to sharing via {:?}, edge_pos={:.0}",
                side, edge_pos
            );
            self.status.sharing_state =
                axis_domain::models::continuity::SharingState::PendingSwitch;
            connection.send_message(Message::SwitchTransition { side, edge_pos });
            self.push();
        }
    }

    async fn handle_send_input(
        &mut self,
        event: &InputEvent,
        connection: &mut dyn ContinuityNetworkPort,
        capture: &mut dyn ContinuityCapturePort,
    ) {
        if !matches!(
            self.status.sharing_state,
            axis_domain::models::continuity::SharingState::Sharing { .. }
        ) {
            return;
        }
        match event {
            InputEvent::CursorMove { dx, dy } => {
                connection.send_message(Message::CursorMove { dx: *dx, dy: *dy });
            }
            InputEvent::KeyPress { key, state } => {
                connection.send_message(Message::KeyPress {
                    key: *key,
                    state: *state,
                });
            }
            InputEvent::KeyRelease { key } => {
                connection.send_message(Message::KeyRelease { key: *key });
            }
            InputEvent::PointerButton { button, state } => {
                connection.send_message(Message::PointerButton {
                    button: *button,
                    state: *state,
                });
            }
            InputEvent::PointerAxis { dx, dy } => {
                connection.send_message(Message::PointerAxis { dx: *dx, dy: *dy });
            }
            InputEvent::EmergencyExit => {
                info!("[continuity] emergency exit via SendInput");
                self.status.sharing_state = axis_domain::models::continuity::SharingState::Idle;
                capture.stop_capture();
                connection.send_message(Message::TransitionCancel);
                self.push();
                let _ = capture.prepare();
            }
        }
    }

    pub(crate) async fn handle_set_peer_arrangement(
        &mut self,
        arrangement: PeerArrangement,
        connection: &mut dyn ContinuityNetworkPort,
    ) {
        if let Some(conn) = &self.status.active_connection {
            let peer_id = conn.peer_id.clone();
            let config = self.status.peer_configs.entry(peer_id.clone()).or_default();
            config.arrangement = arrangement;
            config.version += 1;
            let version = config.version;

            if let Some(known) = self.known_peers.peers.get_mut(&peer_id) {
                known.arrangement_side = KnownPeerArrangementSide::from(arrangement.side);
                match arrangement.side {
                    Side::Left | Side::Right => {
                        known.arrangement_y = arrangement.offset;
                    }
                    Side::Top | Side::Bottom => {
                        known.arrangement_x = arrangement.offset;
                    }
                }
                known_peers::save_known_peers(&self.known_peers);
            }

            info!(
                "[continuity] updated config for peer {}: {:?} (v{})",
                conn.peer_name, arrangement, version
            );

            connection.send_message(Message::ConfigSync {
                arrangement: arrangement.side,
                offset: arrangement.offset,
                clipboard: config.clipboard,
                audio: config.audio,
                audio_direction: config.audio_direction,
                drag_drop: config.drag_drop,
                version,
            });
        }
        self.push();
    }

    pub(crate) async fn handle_update_peer_configs(
        &mut self,
        configs: HashMap<String, PeerConfig>,
        connection: &mut dyn ContinuityNetworkPort,
        clipboard: &mut dyn ContinuityClipboardPort,
        clipboard_tx: &Sender<ClipboardEvent>,
        audio_stream_mgr: &dyn ContinuityAudioPort,
    ) {
        let mut changed = false;
        for (id, config) in configs {
            let entry = self.status.peer_configs.entry(id.clone()).or_default();
            let clipboard_toggled = entry.clipboard != config.clipboard;
            let audio_toggled = entry.audio != config.audio
                || entry.audio_direction != config.audio_direction
                || entry.capture_device != config.capture_device;
            if entry.version < config.version
                || (entry.version == config.version && entry.arrangement != config.arrangement)
            {
                *entry = config.clone();
                changed = true;
            } else if entry.clipboard != config.clipboard
                || entry.audio != config.audio
                || entry.audio_direction != config.audio_direction
                || entry.capture_device != config.capture_device
                || entry.drag_drop != config.drag_drop
                || entry.auto_connect != config.auto_connect
            {
                entry.clipboard = config.clipboard;
                entry.audio = config.audio;
                entry.audio_direction = config.audio_direction;
                entry.capture_device = config.capture_device.clone();
                entry.playback_device = config.playback_device.clone();
                entry.drag_drop = config.drag_drop;
                entry.auto_connect = config.auto_connect;
                changed = true;
            }

            if let Some(known) = self.known_peers.peers.get_mut(&id) {
                known.clipboard = config.clipboard;
                known.audio = config.audio;
                known.audio_direction = config.audio_direction;
                known.drag_drop = config.drag_drop;
                known.auto_connect = config.auto_connect;
                known.arrangement_side = KnownPeerArrangementSide::from(config.arrangement.side);
                match config.arrangement.side {
                    Side::Left | Side::Right => {
                        known.arrangement_y = config.arrangement.offset;
                    }
                    Side::Top | Side::Bottom => {
                        known.arrangement_x = config.arrangement.offset;
                    }
                }
            }

            let is_peer_active = self
                .status
                .active_connection
                .as_ref()
                .is_some_and(|conn| conn.peer_id == id);

            info!(
                "[continuity] update_peer_configs: id={} active={} clipboard={} audio={} dir={:?} drag_drop={} auto_connect={} version={}",
                &id[..8.min(id.len())],
                is_peer_active,
                config.clipboard,
                config.audio,
                config.audio_direction,
                config.drag_drop,
                config.auto_connect,
                config.version,
            );

            if is_peer_active {
                entry.version = entry.version.saturating_add(1);
                let version = entry.version;

                info!(
                    "[continuity] sending ConfigSync to {}: v{} clipboard={} audio={} dir={:?} drag_drop={}",
                    &id[..8.min(id.len())],
                    version,
                    config.clipboard,
                    config.audio,
                    config.audio_direction,
                    config.drag_drop,
                );

                connection.send_message(Message::ConfigSync {
                    arrangement: config.arrangement.side,
                    offset: config.arrangement.offset,
                    clipboard: config.clipboard,
                    audio: config.audio,
                    audio_direction: config.audio_direction,
                    drag_drop: config.drag_drop,
                    version,
                });

                if clipboard_toggled {
                    if config.clipboard {
                        let _ = clipboard.start_monitoring(clipboard_tx.clone());
                    } else {
                        clipboard.stop_monitoring();
                    }
                }

                if audio_toggled {
                    if config.audio || config.audio_direction.should_capture() {
                        if let Some(write_tx) = connection.active_write_tx() {
                            let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
                            let target = config.capture_device.clone();
                            audio_stream_mgr
                                .start_capture(target.as_deref(), audio_tx)
                                .await;
                            let cipher = self.cipher_arc();
                            tokio::spawn(async move {
                                while let Some(chunk) = audio_rx.recv().await {
                                    let encrypted = {
                                        let mut guard = cipher.lock().unwrap();
                                        if let Some(ref mut c) = *guard {
                                            c.encrypt(&chunk)
                                        } else {
                                            chunk
                                        }
                                    };
                                    if write_tx
                                        .send(Message::AudioChunk {
                                            channel_id: 0,
                                            pcm_data: encrypted,
                                        })
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                            });
                        }
                    } else {
                        audio_stream_mgr.stop_capture().await;
                    }
                }
            }
        }
        if changed {
            known_peers::save_known_peers(&self.known_peers);
            self.push();
        }
    }

    async fn handle_switch_to_receiving(
        &mut self,
        side: Side,
        connection: &mut dyn ContinuityNetworkPort,
        injection: &mut dyn ContinuityInjectionPort,
    ) {
        use axis_domain::models::continuity::SharingState;

        let virtual_pos = match &self.status.sharing_state {
            SharingState::Sharing { virtual_pos, .. } => Some(*virtual_pos),
            SharingState::PendingSwitch => None,
            _ => None,
        };
        if virtual_pos.is_some() || matches!(self.status.sharing_state, SharingState::PendingSwitch)
        {
            let edge_pos = match side {
                Side::Left | Side::Right => virtual_pos.map(|v| v.1).unwrap_or(0.0),
                Side::Top | Side::Bottom => virtual_pos.map(|v| v.0).unwrap_or(0.0),
            };
            info!(
                "[continuity] switching to Receiving via {:?}, edge_pos={:.0}",
                side, edge_pos
            );

            self.status.sharing_state = SharingState::Receiving;

            if let Err(e) = injection.start_injection() {
                error!("[continuity] failed to start injection for switch: {e}");
            }

            if let Err(e) = injection.warp(
                side,
                edge_pos,
                self.status.screen_width,
                self.status.screen_height,
            ) {
                error!("[continuity] failed to warp cursor for switch: {e}");
            }

            connection.send_message(Message::SwitchConfirm { side, edge_pos });
            self.push();
        }
    }
}
