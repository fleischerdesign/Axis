use std::collections::HashMap;
use std::pin::Pin;
use std::time::{Duration, Instant};
use uuid::Uuid;

use async_channel::{bounded, Receiver, Sender};
use axis_domain::models::continuity::{
    ActiveConnectionInfo, ContinuityStatus, InputEvent, Message, PeerArrangement, PeerConfig,
    PendingPin, ReconnectState, SharingState, Side,
};
use log::{error, info, warn};

use super::clipboard::{ClipboardEvent, ClipboardSync, WaylandClipboard};
use super::connection::{ConnectionEvent, ConnectionProvider, TcpConnectionProvider};
use super::discovery::{AvahiDiscovery, DiscoveryEvent, DiscoveryProvider};
use super::input::{
    EvdevCapture, InputCapture, InputInjection, InternalInputEvent, WaylandInjection,
};
use super::known_peers::{self, KnownPeer, KnownPeerArrangementSide, KnownPeersStore};
use super::proto;

pub const CONTINUITY_PORT: u16 = 7391;
const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const CONNECTION_TIMEOUT_SECS: u64 = 15;
const RECONNECT_MAX_ATTEMPTS: u32 = 5;
const RECONNECT_BASE_DELAY_MS: u64 = 1000;
const PIN_EXPIRY_SECS: u64 = 30;
const VIRTUAL_POS_BUFFER: f64 = 40.0;

pub enum ContinuityCmd {
    SetEnabled(bool),
    ConnectToPeer(String),
    Unpair(String),
    ConfirmPin,
    RejectPin,
    Disconnect,
    CancelReconnect,
    ForceLocal,
    StartSharing(Side, f64),
    StopSharing(f64),
    SendInput(InputEvent),
    SetPeerArrangement(PeerArrangement),
    UpdatePeerConfigs(HashMap<String, PeerConfig>),
    SwitchToReceiving(Side),
}

pub struct ContinuityInner {
    status_tx: tokio::sync::watch::Sender<ContinuityStatus>,
    status: ContinuityStatus,
    connected_at: Option<Instant>,
    pin_created_at: Option<Instant>,
    last_message_at: Option<Instant>,
    is_initiating: bool,
    pending_peer: Option<(String, String)>,
    last_transition_at: Instant,
    switch_tx: Option<Sender<ContinuityCmd>>,
    known_peers: KnownPeersStore,
}

impl ContinuityInner {
    pub fn new(status_tx: tokio::sync::watch::Sender<ContinuityStatus>) -> Self {
        let known_peers = known_peers::load_known_peers();
        let mut status = ContinuityStatus {
            device_id: persistent_device_id(),
            device_name: known_peers::hostname(),
            ..ContinuityStatus::default()
        };
        for (id, known_peer) in &known_peers.peers {
            status.peer_configs.insert(id.clone(), known_peer.to_peer_config());
        }
        Self {
            status_tx,
            status,
            connected_at: None,
            pin_created_at: None,
            last_message_at: None,
            is_initiating: false,
            pending_peer: None,
            last_transition_at: Instant::now() - Duration::from_secs(10),
            switch_tx: None,
            known_peers,
        }
    }

    fn push(&self) {
        let mut status = self.status.clone();
        if let (Some(conn), Some(started)) = (&mut status.active_connection, &self.connected_at)
        {
            conn.connected_secs = started.elapsed().as_secs();
        }
        let _ = self.status_tx.send(status);
    }

    fn persist_known_peers(&self) {
        let peers: HashMap<_, _> = self
            .status
            .peer_configs
            .iter()
            .filter(|(_, c)| c.trusted)
            .filter_map(|(id, config)| {
                let existing = self.known_peers.peers.get(id);
                let (device_name, hostname, address, address_v6) = existing
                    .map(|kp| {
                        (
                            kp.device_name.clone(),
                            kp.hostname.clone(),
                            kp.address.clone(),
                            kp.address_v6.clone(),
                        )
                    })
                    .or_else(|| {
                        self.status
                            .peers
                            .iter()
                            .find(|p| &p.device_id == id)
                            .map(|p| {
                                (
                                    p.device_name.clone(),
                                    p.hostname.clone(),
                                    p.address.to_string(),
                                    p.address_v6.map(|a| a.to_string()),
                                )
                            })
                    })
                    .unwrap_or_else(|| {
                        (String::new(), String::new(), String::new(), None)
                    });

                let (arrangement_side, arrangement_x, arrangement_y) =
                    match config.arrangement.side {
                        Side::Left | Side::Right => (
                            KnownPeerArrangementSide::from(config.arrangement.side),
                            0,
                            config.arrangement.offset,
                        ),
                        Side::Top | Side::Bottom => (
                            KnownPeerArrangementSide::from(config.arrangement.side),
                            config.arrangement.offset,
                            0,
                        ),
                    };

                let known_peer = KnownPeer {
                    device_id: id.clone(),
                    device_name,
                    hostname,
                    address,
                    address_v6,
                    trusted: config.trusted,
                    clipboard: config.clipboard,
                    audio: config.audio,
                    drag_drop: config.drag_drop,
                    arrangement_side,
                    arrangement_x,
                    arrangement_y,
                };
                Some((id.clone(), known_peer))
            })
            .collect();

        known_peers::save_known_peers(&KnownPeersStore { peers });
    }

    fn remote_screen(&self) -> (i32, i32) {
        self.status
            .remote_screen
            .unwrap_or((self.status.screen_width, self.status.screen_height))
    }

    fn init_virtual_pos(
        entry_side: Side,
        edge_pos: f64,
        remote_w: i32,
        remote_h: i32,
    ) -> (f64, f64) {
        let (rw, rh) = (remote_w as f64, remote_h as f64);
        let buffer = VIRTUAL_POS_BUFFER;
        match entry_side {
            Side::Right => (buffer, edge_pos.clamp(0.0, rh)),
            Side::Left => (rw - buffer, edge_pos.clamp(0.0, rh)),
            Side::Bottom => (edge_pos.clamp(0.0, rw), buffer),
            Side::Top => (edge_pos.clamp(0.0, rw), rh - buffer),
        }
    }

    pub async fn run(&mut self, cmd_rx: Receiver<ContinuityCmd>) {
        use tokio::select;
        use tokio::time::{interval, Duration};

        let (discovery_tx, discovery_rx) = bounded::<DiscoveryEvent>(32);
        let (conn_tx, conn_rx) = bounded::<ConnectionEvent>(64);
        let (clipboard_tx, clipboard_rx) = bounded::<ClipboardEvent>(32);
        let (input_tx, input_rx) = bounded::<InternalInputEvent>(128);
        let (switch_tx, switch_rx) = bounded::<ContinuityCmd>(8);
        self.switch_tx = Some(switch_tx);

        let mut discovery = AvahiDiscovery::new();
        let mut connection = TcpConnectionProvider::new();
        let mut clipboard = WaylandClipboard::new();
        let mut injection = WaylandInjection::new();
        let mut capture = EvdevCapture::new();
        let mut heartbeat = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        let mut reconnect_sleep: Option<Pin<Box<tokio::time::Sleep>>> = None;

        if let Ok(mut sock) = niri_ipc::socket::Socket::connect() {
            if let Ok(Ok(niri_ipc::Response::Outputs(outputs))) =
                sock.send(niri_ipc::Request::Outputs)
            {
                let mut best: Option<(i32, i32)> = None;
                let mut best_area: u64 = 0;
                for output in outputs.values() {
                    if let Some(logical) = &output.logical {
                        let w = logical.width as i32;
                        let h = logical.height as i32;
                        let covers_origin = logical.x <= 0
                            && logical.y <= 0
                            && logical.x + w > 0
                            && logical.y + h > 0;
                        let area = (w as u64) * (h as u64);
                        if covers_origin || area > best_area {
                            best = Some((w, h));
                            best_area = area;
                        }
                    }
                }
                if let Some((w, h)) = best {
                    info!(
                        "[continuity] detected primary output: {}x{} (logical)",
                        w, h
                    );
                    self.status.screen_width = w;
                    self.status.screen_height = h;
                }
            }
        }

        info!(
            "[continuity] service started, device: {}",
            self.status.device_name
        );

        loop {
            let is_connected = self.status.active_connection.is_some();

            select! {
                Ok(cmd) = cmd_rx.recv() => {
                    self.handle_cmd(
                        cmd,
                        &mut discovery,
                        &mut connection,
                        &mut clipboard,
                        &mut injection,
                        &mut capture,
                        &discovery_tx,
                        &conn_tx,
                        &clipboard_tx,
                    ).await;
                }
                Ok(cmd) = switch_rx.recv() => {
                    self.handle_cmd(
                        cmd,
                        &mut discovery,
                        &mut connection,
                        &mut clipboard,
                        &mut injection,
                        &mut capture,
                        &discovery_tx,
                        &conn_tx,
                        &clipboard_tx,
                    ).await;
                }
                Ok(event) = discovery_rx.recv() => {
                    self.handle_discovery_event(event).await;
                }
                Ok(event) = conn_rx.recv() => {
                    let new_sleep = self.handle_connection_event(
                        event,
                        &mut connection,
                        &mut clipboard,
                        &mut injection,
                        &mut capture,
                        &clipboard_tx,
                        &input_tx,
                    ).await;
                    if let Some(sleep) = new_sleep {
                        reconnect_sleep = Some(sleep);
                    }
                }
                Ok(event) = clipboard_rx.recv() => {
                    self.handle_clipboard_event(event, &connection).await;
                }
                Ok(event) = input_rx.recv() => {
                    self.handle_input_capture_event(event, &connection, &mut capture).await;
                }
                _ = heartbeat.tick(), if is_connected => {
                    self.handle_heartbeat(&mut connection, &mut capture);
                }
                _ = async { reconnect_sleep.as_mut()?.await; Some(()) }, if reconnect_sleep.is_some() => {
                    reconnect_sleep = self.handle_reconnect_attempt(&mut connection, &conn_tx);
                }
            }
        }
    }

    async fn handle_cmd(
        &mut self,
        cmd: ContinuityCmd,
        discovery: &mut AvahiDiscovery,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
        clipboard_tx: &Sender<ClipboardEvent>,
    ) {
        match cmd {
            ContinuityCmd::SetEnabled(on) => {
                self.handle_set_enabled(
                    on, discovery, connection, clipboard, injection, capture,
                    discovery_tx, conn_tx,
                )
                .await;
            }
            ContinuityCmd::ConnectToPeer(peer_id) => {
                self.handle_connect_to_peer(&peer_id, connection, discovery_tx, conn_tx)
                    .await;
            }
            ContinuityCmd::ConfirmPin => {
                self.handle_confirm_pin(connection, clipboard, injection, capture, clipboard_tx)
                    .await;
            }
            ContinuityCmd::RejectPin => {
                self.handle_reject_pin(connection, clipboard, injection, capture)
                    .await;
            }
            ContinuityCmd::Disconnect => {
                self.handle_disconnect(connection, clipboard, injection, capture)
                    .await;
            }
            ContinuityCmd::CancelReconnect => {
                self.handle_cancel_reconnect().await;
            }
            ContinuityCmd::Unpair(peer_id) => {
                self.handle_unpair(&peer_id, connection, clipboard, injection, capture)
                    .await;
            }
            ContinuityCmd::ForceLocal => {
                self.handle_force_local(capture, connection).await;
            }
            ContinuityCmd::StartSharing(side, local_edge_pos) => {
                self.handle_start_sharing(side, local_edge_pos, connection)
                    .await;
            }
            ContinuityCmd::StopSharing(edge_pos) => {
                self.handle_stop_sharing(edge_pos, connection, capture)
                    .await;
            }
            ContinuityCmd::SendInput(event) => {
                self.handle_send_input(&event, connection, capture).await;
            }
            ContinuityCmd::SetPeerArrangement(arrangement) => {
                self.handle_set_peer_arrangement(arrangement, connection)
                    .await;
            }
            ContinuityCmd::UpdatePeerConfigs(configs) => {
                self.handle_update_peer_configs(configs).await;
            }
            ContinuityCmd::SwitchToReceiving(side) => {
                self.handle_switch_to_receiving(side, connection, injection)
                    .await;
            }
        }
    }

    // ── cmd handlers ────────────────────────────────────────────────────

    async fn handle_set_enabled(
        &mut self,
        on: bool,
        discovery: &mut AvahiDiscovery,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
    ) {
        if self.status.enabled == on {
            return;
        }
        self.status.enabled = on;
        if on {
            info!("[continuity] enabled");
            if let Err(e) = discovery.register(&self.status.device_name, CONTINUITY_PORT) {
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
            self.status.peers.clear();
            self.status.active_connection = None;
            self.connected_at = None;
            self.status.sharing_state = SharingState::Idle;
            self.status.pending_pin = None;
            self.pin_created_at = None;
            self.last_message_at = None;
        }
        self.push();
    }

    async fn handle_connect_to_peer(
        &mut self,
        peer_id: &str,
        connection: &mut TcpConnectionProvider,
        _discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
    ) {
        if let Some(peer) = self
            .status
            .peers
            .iter()
            .find(|p| p.device_id == peer_id)
        {
            let name = peer.device_name.clone();
            let addr_v4 = peer.address;
            let addr_v6 = peer.address_v6;

            info!("[continuity] connecting to {name}");
            self.is_initiating = true;
            self.pending_peer = Some((peer.device_id.clone(), name.clone()));

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
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        _capture: &mut EvdevCapture,
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

                if let Err(e) = injection.start() {
                    error!("[continuity] failed to start input injection: {e}");
                }
            }
        }
        self.push();
    }

    async fn handle_reject_pin(
        &mut self,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] PIN rejected");
        self.status.pending_pin = None;
        self.pin_created_at = None;
        connection.send_message(Message::Disconnect {
            reason: "PIN rejected".to_string(),
        });
        connection.disconnect_active();
        clipboard.stop_monitoring();
        injection.stop();
        capture.stop();
        self.status.active_connection = None;
        self.connected_at = None;
        self.status.sharing_state = SharingState::Idle;
        self.push();
    }

    async fn handle_disconnect(
        &mut self,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
    ) {
        info!("[continuity] disconnecting");
        connection.disconnect_active();
        clipboard.stop_monitoring();
        injection.stop();
        capture.stop();
        self.status.active_connection = None;
        self.connected_at = None;
        self.status.sharing_state = SharingState::Idle;
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
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
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
            injection.stop();
            capture.stop();
            self.status.active_connection = None;
            self.connected_at = None;
            self.status.sharing_state = SharingState::Idle;
            self.last_message_at = None;
        }
        self.push();
    }

    async fn handle_force_local(
        &mut self,
        capture: &mut EvdevCapture,
        connection: &mut TcpConnectionProvider,
    ) {
        if !self.status.sharing_state.is_active() {
            info!("[continuity] forcing cursor back to local");
            self.status.sharing_state = SharingState::Idle;
            capture.stop();
            connection.send_message(Message::TransitionCancel);
            self.push();
            let _ = capture.prepare();
        }
    }

    async fn handle_start_sharing(
        &mut self,
        side: Side,
        local_edge_pos: f64,
        connection: &mut TcpConnectionProvider,
    ) {
        if self.status.active_connection.is_some()
            && self.status.sharing_state == SharingState::Idle
        {
            if self.last_transition_at.elapsed() < Duration::from_millis(500) {
                return;
            }

            let arrangement = self.status.active_peer_config().arrangement;
            let remote_edge_pos = arrangement.local_to_remote_edge(local_edge_pos);
            info!(
                "[continuity] initiating sharing via {:?}, local_pos={:.0} remote_pos={:.0}",
                side, local_edge_pos, remote_edge_pos
            );
            self.status.sharing_state = SharingState::Pending {
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
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        if matches!(
            &self.status.sharing_state,
            SharingState::Sharing { .. } | SharingState::Pending { .. }
        ) {
            info!("[continuity] stopping sharing");
            self.status.sharing_state = SharingState::Idle;
            capture.stop();
            connection.send_message(Message::TransitionCancel);
            self.push();
            let _ = capture.prepare();
        } else if matches!(self.status.sharing_state, SharingState::Receiving) {
            let side = self.status.active_peer_config().arrangement.side;
            info!(
                "[continuity] requesting switch back to sharing via {:?}, edge_pos={:.0}",
                side, edge_pos
            );
            self.status.sharing_state = SharingState::PendingSwitch;
            connection.send_message(Message::SwitchTransition { side, edge_pos });
            self.push();
        }
    }

    async fn handle_send_input(
        &mut self,
        event: &InputEvent,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        if !matches!(self.status.sharing_state, SharingState::Sharing { .. }) {
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
                self.status.sharing_state = SharingState::Idle;
                capture.stop();
                connection.send_message(Message::TransitionCancel);
                self.push();
                let _ = capture.prepare();
            }
        }
    }

    async fn handle_set_peer_arrangement(
        &mut self,
        arrangement: PeerArrangement,
        connection: &mut TcpConnectionProvider,
    ) {
        if let Some(conn) = &self.status.active_connection {
            let peer_id = conn.peer_id.clone();
            let config = self
                .status
                .peer_configs
                .entry(peer_id.clone())
                .or_default();
            config.arrangement = arrangement;
            config.version += 1;
            let version = config.version;

            if let Some(known) = self.known_peers.peers.get_mut(&peer_id) {
                known.arrangement_side =
                    KnownPeerArrangementSide::from(arrangement.side);
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
                drag_drop: config.drag_drop,
                version,
            });
        }
        self.push();
    }

    async fn handle_update_peer_configs(
        &mut self,
        configs: HashMap<String, PeerConfig>,
    ) {
        let mut changed = false;
        for (id, config) in configs {
            let entry = self.status.peer_configs.entry(id.clone()).or_default();
            if entry.version < config.version
                || (entry.version == config.version
                    && entry.arrangement != config.arrangement)
            {
                *entry = config.clone();
                changed = true;
            } else if entry.clipboard != config.clipboard
                || entry.audio != config.audio
                || entry.drag_drop != config.drag_drop
            {
                entry.clipboard = config.clipboard;
                entry.audio = config.audio;
                entry.drag_drop = config.drag_drop;
                changed = true;
            }

            if let Some(known) = self.known_peers.peers.get_mut(&id) {
                known.clipboard = config.clipboard;
                known.audio = config.audio;
                known.drag_drop = config.drag_drop;
                known.arrangement_side =
                    KnownPeerArrangementSide::from(config.arrangement.side);
                match config.arrangement.side {
                    Side::Left | Side::Right => {
                        known.arrangement_y = config.arrangement.offset;
                    }
                    Side::Top | Side::Bottom => {
                        known.arrangement_x = config.arrangement.offset;
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
        connection: &mut TcpConnectionProvider,
        injection: &mut WaylandInjection,
    ) {
        let virtual_pos = match &self.status.sharing_state {
            SharingState::Sharing { virtual_pos, .. } => Some(*virtual_pos),
            SharingState::PendingSwitch => None,
            _ => None,
        };
        if virtual_pos.is_some()
            || matches!(self.status.sharing_state, SharingState::PendingSwitch)
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

            if let Err(e) = injection.start() {
                error!(
                    "[continuity] failed to start injection for switch: {e}"
                );
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

    // ── event handlers ──────────────────────────────────────────────────

    async fn handle_discovery_event(&mut self, event: DiscoveryEvent) {
        match event {
            DiscoveryEvent::PeerFound(peer) => {
                if peer.hostname == self.status.device_name {
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

    fn handle_heartbeat(
        &mut self,
        connection: &mut TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        if let Some(last) = self.last_message_at {
            let timeout =
                if matches!(self.status.sharing_state, SharingState::Receiving) {
                    Duration::from_secs(5)
                } else if matches!(
                    &self.status.sharing_state,
                    SharingState::Pending { .. } | SharingState::PendingSwitch
                ) {
                    Duration::from_secs(5)
                } else {
                    Duration::from_secs(CONNECTION_TIMEOUT_SECS)
                };

            if last.elapsed() > timeout {
                warn!(
                    "[continuity] peer timed out (no message for {:?})",
                    timeout
                );
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
                warn!(
                    "[continuity] PIN expired ({}s timeout)",
                    PIN_EXPIRY_SECS
                );
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

    async fn handle_clipboard_event(
        &mut self,
        event: ClipboardEvent,
        connection: &TcpConnectionProvider,
    ) {
        match event {
            ClipboardEvent::ContentChanged { content, mime_type } => {
                if self.status.active_connection.is_some() {
                    info!("[continuity] clipboard changed, sending to peer");
                    connection.send_message(Message::ClipboardUpdate {
                        content,
                        mime_type,
                    });
                }
            }
        }
    }

    // ── connection handlers ─────────────────────────────────────────────

    async fn handle_connection_event(
        &mut self,
        event: ConnectionEvent,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        clipboard_tx: &Sender<ClipboardEvent>,
        input_tx: &Sender<InternalInputEvent>,
    ) -> Option<Pin<Box<tokio::time::Sleep>>> {
        match event {
            ConnectionEvent::IncomingConnection { addr, write_tx } => {
                self.handle_incoming_connection(addr, write_tx, connection)
                    .await;
                None
            }
            ConnectionEvent::HandshakeComplete { .. } => None,
            ConnectionEvent::Disconnected { reason } => {
                self.handle_disconnected(
                    reason,
                    connection,
                    clipboard,
                    injection,
                    capture,
                )
                .await
            }
            ConnectionEvent::MessageReceived(msg) => {
                self.handle_message_received(
                    msg,
                    connection,
                    clipboard,
                    injection,
                    capture,
                    clipboard_tx,
                    input_tx,
                )
                .await;
                None
            }
            ConnectionEvent::Error(e) => {
                error!("[continuity] connection error: {e}");
                None
            }
        }
    }

    async fn handle_incoming_connection(
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

    async fn handle_disconnected(
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

    async fn handle_message_received(
        &mut self,
        msg: Message,
        connection: &mut TcpConnectionProvider,
        clipboard: &mut WaylandClipboard,
        injection: &mut WaylandInjection,
        capture: &mut EvdevCapture,
        clipboard_tx: &Sender<ClipboardEvent>,
        input_tx: &Sender<InternalInputEvent>,
    ) {
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
                    connection,
                    injection,
                    capture,
                )
                .await;
            }
            Message::PinRequest { pin } => {
                self.handle_pin_request(pin).await;
            }
            Message::PinConfirm { pin } => {
                self.handle_pin_confirm(
                    pin,
                    connection,
                    clipboard,
                    injection,
                    capture,
                    clipboard_tx,
                )
                .await;
            }
            Message::ClipboardUpdate {
                content,
                mime_type,
            } => {
                if let Err(e) = clipboard.set_content(&content, &mime_type) {
                    error!("[continuity] failed to set clipboard: {e}");
                }
            }
            Message::CursorMove { .. }
            | Message::KeyPress { .. }
            | Message::KeyRelease { .. }
            | Message::PointerButton { .. }
            | Message::PointerAxis { .. } => {
                let _ = injection.inject(&msg);
            }
            Message::ScreenInfo { width, height } => {
                self.handle_screen_info(width, height, connection, capture)
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
                    arrangement,
                    offset,
                    cb,
                    audio,
                    drag_drop,
                    version,
                    connection,
                )
                .await;
            }
            Message::EdgeTransition { side, edge_pos } => {
                self.handle_edge_transition(side, edge_pos, connection, injection)
                    .await;
            }
            Message::TransitionAck { accepted } => {
                self.handle_transition_ack(
                    accepted,
                    connection,
                    capture,
                    input_tx,
                )
                .await;
            }
            Message::TransitionCancel => {
                self.handle_transition_cancel().await;
            }
            Message::SwitchTransition { side, edge_pos: _ } => {
                self.handle_switch_transition(side, connection).await;
            }
            Message::SwitchConfirm { side, edge_pos } => {
                self.handle_switch_confirm(
                    side,
                    edge_pos,
                    connection,
                    capture,
                    input_tx,
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
                    connection,
                    clipboard,
                    injection,
                    capture,
                )
                .await;
            }
        }
    }

    async fn handle_hello(
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
                    error!(
                        "[continuity] failed to start input injection: {e}"
                    );
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
                info!(
                    "[continuity] trusted peer connected (incoming), skipping PIN"
                );
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
                    error!(
                        "[continuity] failed to start input injection: {e}"
                    );
                }
                let _ = capture.prepare();
            } else {
                let pin = format!("{:06}", rand::random_range(0..1_000_000));
                info!(
                    "[continuity] incoming pairing request, generating PIN: {pin}"
                );
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

    async fn handle_pin_request(&mut self, pin: String) {
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

    async fn handle_pin_confirm(
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

                if let Err(e) = clipboard.start_monitoring(clipboard_tx.clone())
                {
                    error!(
                        "[continuity] failed to start clipboard monitoring: {e}"
                    );
                }

                if let Err(e) = injection.start() {
                    error!(
                        "[continuity] failed to start input injection: {e}"
                    );
                }

                self.push();
                let _ = capture.prepare();
            } else {
                warn!("[continuity] peer sent incorrect PIN confirmation");
                connection.disconnect_active();
            }
        }
    }

    async fn handle_screen_info(
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

    async fn handle_config_sync(
        &mut self,
        arrangement: Side,
        offset: i32,
        clipboard: bool,
        audio: bool,
        drag_drop: bool,
        version: u64,
        _connection: &mut TcpConnectionProvider,
    ) {
        if let Some(conn) = &self.status.active_connection {
            let peer_id = conn.peer_id.clone();
            let config = self.status.peer_configs.entry(peer_id).or_default();

            if version > config.version {
                info!(
                    "[continuity] adopting newer config from peer (v{} > v{}): {:?} offset {} clipboard={} audio={} dnd={}",
                    version, config.version, arrangement, offset, clipboard, audio, drag_drop
                );

                config.arrangement = PeerArrangement {
                    side: arrangement.opposite(),
                    offset: -offset,
                };
                config.clipboard = clipboard;
                config.audio = audio;
                config.drag_drop = drag_drop;
                config.version = version;
                self.push();
            } else {
                info!(
                    "[continuity] ignoring older/same config from peer (v{} <= v{})",
                    version, config.version
                );
            }
        }
    }

    async fn handle_edge_transition(
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

    async fn handle_transition_ack(
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
                let virtual_pos =
                    Self::init_virtual_pos(entry_side, edge_pos, rw, rh);
                info!(
                    "[continuity] virtual_pos initialized to ({:.0}, {:.0})",
                    virtual_pos.0, virtual_pos.1
                );

                if let Err(e) = capture.start(input_tx.clone()) {
                    error!(
                        "[continuity] failed to start input capture: {e}"
                    );
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

    async fn handle_transition_cancel(&mut self) {
        info!("[continuity] forcing cursor back to local");
        self.status.sharing_state = SharingState::Idle;
        self.push();
    }

    async fn handle_switch_transition(
        &mut self,
        side: Side,
        connection: &mut TcpConnectionProvider,
    ) {
        if matches!(self.status.sharing_state, SharingState::Sharing { .. }) {
            info!("[continuity] peer requesting switch via {:?}", side);
            self.status.sharing_state = SharingState::PendingSwitch;
            self.push();
            if let Some(tx) = &self.switch_tx {
                let _ = tx.try_send(ContinuityCmd::SwitchToReceiving(side));
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

    async fn handle_switch_confirm(
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
            let virtual_pos =
                Self::init_virtual_pos(side, edge_pos.max(0.0), rw, rh);
            info!(
                "[continuity] virtual_pos initialized to ({:.0}, {:.0})",
                virtual_pos.0, virtual_pos.1
            );

            if let Err(e) = capture.start(input_tx.clone()) {
                error!(
                    "[continuity] failed to start input capture after switch: {e}"
                );
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

    async fn handle_peer_disconnect(
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

    // ── sharing / input capture handler ─────────────────────────────────

    async fn handle_input_capture_event(
        &mut self,
        event: InternalInputEvent,
        connection: &TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        if !matches!(self.status.sharing_state, SharingState::Sharing { .. }) {
            return;
        }
        match event {
            InternalInputEvent::CursorMove { dx, dy } => {
                let (rw, rh) = self.remote_screen();
                let rw_f = rw as f64;
                let rh_f = rh as f64;

                let SharingState::Sharing {
                    entry_side,
                    virtual_pos: mut vpos,
                } = self.status.sharing_state.clone()
                else {
                    return;
                };

                vpos.0 += dx;
                vpos.1 += dy;
                vpos.0 = vpos.0.clamp(-100.0, rw_f + 100.0);
                vpos.1 = vpos.1.clamp(-100.0, rh_f + 100.0);

                let should_return = match entry_side {
                    Side::Left if vpos.0 > rw_f => true,
                    Side::Right if vpos.0 < 0.0 => true,
                    Side::Top if vpos.1 > rh_f => true,
                    Side::Bottom if vpos.1 < 0.0 => true,
                    _ => false,
                };

                if should_return {
                    info!(
                        "[continuity] return transition at vpos=({:.0},{:.0})",
                        vpos.0, vpos.1
                    );
                    self.status.sharing_state = SharingState::Idle;
                    self.last_transition_at = Instant::now();
                    capture.stop();
                    connection.send_message(Message::TransitionCancel);
                    self.push();
                    let _ = capture.prepare();
                    return;
                }

                self.status.sharing_state = SharingState::Sharing {
                    entry_side,
                    virtual_pos: vpos,
                };
                connection.send_message(Message::CursorMove { dx, dy });
            }
            InternalInputEvent::KeyPress { key, state } => {
                connection.send_message(Message::KeyPress { key, state });
            }
            InternalInputEvent::KeyRelease { key } => {
                connection.send_message(Message::KeyRelease { key });
            }
            InternalInputEvent::PointerButton { button, state } => {
                connection.send_message(Message::PointerButton { button, state });
            }
            InternalInputEvent::PointerAxis { dx, dy } => {
                connection.send_message(Message::PointerAxis { dx, dy });
            }
            InternalInputEvent::EmergencyExit => {
                info!("[continuity] kernel emergency exit requested");
                self.status.sharing_state = SharingState::Idle;
                capture.stop();
                connection.send_message(Message::TransitionCancel);
                self.push();
                let _ = capture.prepare();
            }
        };
    }

    // ── reconnect handlers ──────────────────────────────────────────────

    fn schedule_reconnect(&mut self) -> Option<Pin<Box<tokio::time::Sleep>>> {
        let reconnect = match &self.status.reconnect {
            Some(r) => r.clone(),
            None => return None,
        };

        if reconnect.attempt > reconnect.max_attempts {
            warn!(
                "[continuity] reconnect failed after {} attempts, giving up",
                reconnect.attempt - 1
            );
            self.status.reconnect = None;
            self.push();
            return None;
        }

        let delay_secs = reconnect.delay_secs;
        info!(
            "[continuity] scheduling reconnect for {} (attempt {}/{}, in {}s)",
            reconnect.peer_name,
            reconnect.attempt,
            reconnect.max_attempts,
            delay_secs
        );

        Some(Box::pin(tokio::time::sleep(Duration::from_secs(delay_secs))))
    }

    fn start_reconnect(
        &mut self,
        peer_id: &str,
        peer_name: &str,
    ) -> Option<Pin<Box<tokio::time::Sleep>>> {
        if self.status.reconnect.is_some() {
            return None;
        }
        let attempt = 1;
        let delay_secs = RECONNECT_BASE_DELAY_MS / 1000;
        self.status.reconnect = Some(ReconnectState {
            peer_id: peer_id.to_string(),
            peer_name: peer_name.to_string(),
            attempt,
            max_attempts: RECONNECT_MAX_ATTEMPTS,
            delay_secs,
        });
        self.schedule_reconnect()
    }

    fn handle_reconnect_attempt(
        &mut self,
        connection: &mut TcpConnectionProvider,
        conn_tx: &Sender<ConnectionEvent>,
    ) -> Option<Pin<Box<tokio::time::Sleep>>> {
        let reconnect = match &self.status.reconnect {
            Some(r) => r.clone(),
            None => return None,
        };

        info!(
            "[continuity] reconnect attempt {}/{} for {}",
            reconnect.attempt,
            reconnect.max_attempts,
            reconnect.peer_name
        );

        let peer_info = self
            .status
            .peers
            .iter()
            .find(|p| p.device_id == reconnect.peer_id)
            .cloned();

        if let Some(peer) = peer_info {
            self.is_initiating = true;
            self.pending_peer = Some((
                peer.device_id.clone(),
                peer.device_name.clone(),
            ));

            connection.connect_dual(
                peer.address,
                peer.address_v6,
                conn_tx.clone(),
                self.status.device_id.clone(),
                self.status.device_name.clone(),
            );

            let next_delay =
                RECONNECT_BASE_DELAY_MS * 2u64.pow(reconnect.attempt - 1) / 1000;
            if let Some(ref mut r) = self.status.reconnect {
                r.attempt += 1;
                r.delay_secs = next_delay;
            }
            self.push();

            self.schedule_reconnect()
        } else {
            if let Some(ref mut r) = self.status.reconnect {
                r.delay_secs = 5;
            }
            self.schedule_reconnect()
        }
    }
}

fn persistent_device_id() -> String {
    let path = known_peers::config_dir().join("continuity_id");
    if let Ok(id) = std::fs::read_to_string(&path) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }
    let id = Uuid::new_v4().to_string();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &id);
    id
}
