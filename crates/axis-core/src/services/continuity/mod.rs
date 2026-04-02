pub mod clipboard;
pub mod connection;
pub mod dbus;
pub mod discovery;
pub mod input;
pub mod protocol;

use async_channel::{bounded, Sender};
use std::time::{Instant, Duration};

use super::{Service, ServiceConfig};
use crate::store::ServiceStore;
use clipboard::ClipboardSync;
use input::{InputCapture, InputInjection};
use connection::{ConnectionEvent, ConnectionProvider};
pub use protocol::Side;
use discovery::{DiscoveryEvent, DiscoveryProvider};
use log::{error, info, warn};

// ── Constants ──────────────────────────────────────────────────────────

pub const CONTINUITY_PORT: u16 = 7391;
const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const CONNECTION_TIMEOUT_SECS: u64 = 15;
pub const PIN_LENGTH: usize = 6;

// ── Data Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SharingMode {
    Idle,
    Pending,
    Sharing,
    Receiving,
    PendingSwitch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeerInfo {
    pub device_id: String,
    pub device_name: String,
    pub hostname: String,
    pub address: std::net::SocketAddr,
    pub address_v6: Option<std::net::SocketAddr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveConnectionInfo {
    pub peer_id: String,
    pub peer_name: String,
    pub since: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingPin {
    pub pin: String,
    pub peer_id: String,
    pub peer_name: String,
    pub is_incoming: bool,
}

/// Describes where the peer screen is positioned relative to this screen.
///
/// Think of it like GNOME/Windows display settings: you can arrange monitors
/// next to each other with an optional offset along the shared edge.
///
/// For Left/Right sides, `offset` is vertical (positive = peer is shifted down).
/// For Top/Bottom sides, `offset` is horizontal (positive = peer is shifted right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PeerArrangement {
    pub side: Side,
    pub offset: i32,
}

impl PeerArrangement {
    /// Returns the (start, end) pixel range on our edge where the peer overlaps.
    /// For Left/Right this is a Y range, for Top/Bottom an X range.
    /// Returns None if there is no overlap (screens don't touch).
    pub fn overlap_on_local(&self, local_len: i32, remote_len: i32) -> Option<(i32, i32)> {
        let start = self.offset.max(0);
        let end = (self.offset + remote_len).min(local_len);
        if start < end { Some((start, end)) } else { None }
    }

    /// Returns the (start, end) pixel range on the remote's entry edge
    /// that overlaps with our screen.
    pub fn overlap_on_remote(&self, local_len: i32, remote_len: i32) -> Option<(i32, i32)> {
        let start = (-self.offset).max(0);
        let end = (local_len - self.offset).min(remote_len);
        if start < end { Some((start, end)) } else { None }
    }

    /// Maps a position along our local edge to the corresponding position
    /// on the remote screen. Returns the position in remote screen coords.
    pub fn local_to_remote_edge(&self, local_pos: f64) -> f64 {
        local_pos - self.offset as f64
    }

    /// Maps a position on the remote screen edge back to our local edge.
    pub fn remote_to_local_edge(&self, remote_pos: f64) -> f64 {
        remote_pos + self.offset as f64
    }

    /// Returns the perpendicular length of the local screen for this arrangement side.
    pub fn local_edge_length(&self, screen_w: i32, screen_h: i32) -> i32 {
        match self.side {
            Side::Left | Side::Right => screen_h,
            Side::Top | Side::Bottom => screen_w,
        }
    }
}

impl Default for PeerArrangement {
    fn default() -> Self {
        Self { side: Side::Right, offset: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PeerConfig {
    pub arrangement: PeerArrangement,
    pub version: u64,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            arrangement: PeerArrangement::default(),
            version: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContinuityData {
    pub device_id: String,
    pub device_name: String,
    pub enabled: bool,
    pub peers: Vec<PeerInfo>,
    pub active_connection: Option<ActiveConnectionInfo>,
    pub sharing_mode: SharingMode,
    pub pending_pin: Option<PendingPin>,
    pub peer_configs: std::collections::HashMap<String, PeerConfig>,
    pub screen_width: i32,
    pub screen_height: i32,
    pub remote_screen: Option<(i32, i32)>,
}

impl ContinuityData {
    pub fn active_peer_config(&self) -> PeerConfig {
        if let Some(conn) = &self.active_connection {
            self.peer_configs.get(&conn.peer_id).cloned().unwrap_or_default()
        } else {
            PeerConfig::default()
        }
    }
}

impl Default for ContinuityData {
    fn default() -> Self {
        Self {
            device_id: persistent_device_id(),
            device_name: hostname(),
            enabled: false,
            peers: Vec::new(),
            active_connection: None,
            sharing_mode: SharingMode::Idle,
            pending_pin: None,
            peer_configs: std::collections::HashMap::new(),
            screen_width: 1920,
            screen_height: 1080,
            remote_screen: None,
        }
    }
}

// ── Commands ───────────────────────────────────────────────────────────

pub enum ContinuityCmd {
    SetEnabled(bool),
    ConnectToPeer(String),
    ConfirmPin,
    RejectPin,
    Disconnect,
    ForceLocal,
    StartDiscovery,
    StopDiscovery,
    /// Start sharing cursor to peer. `edge_pos` is the position along the
    /// local edge (absolute screen coords) where the cursor crossed.
    StartSharing(Side, f64),
    StopSharing(f64),
    SendInput(protocol::Message),
    SetPeerArrangement(PeerArrangement),
    SetScreenSize(i32, i32),
    SwitchToReceiving(Side),
}

// ── Service ────────────────────────────────────────────────────────────

pub struct ContinuityService;

impl Service for ContinuityService {
    type Data = ContinuityData;
    type Cmd = ContinuityCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(32);
        let (cmd_tx, cmd_rx) = bounded(32);
        let (switch_tx, switch_rx) = bounded::<ContinuityCmd>(8);

        tokio::spawn(async move {
            let mut service = ContinuityInner::new(data_tx);
            service.switch_tx = Some(switch_tx);
            service.run(cmd_rx, switch_rx).await;
        });

        let store = ServiceStore::new(data_rx, ContinuityData::default());
        (store, cmd_tx)
    }
}

impl ServiceConfig for ContinuityService {
    fn get_enabled(data: &ContinuityData) -> bool { data.enabled }
    fn cmd_set_enabled(on: bool) -> ContinuityCmd { ContinuityCmd::SetEnabled(on) }
}

// ── Internal State ─────────────────────────────────────────────────────

struct ContinuityInner {
    data_tx: Sender<ContinuityData>,
    data: ContinuityData,
    last_message_at: Option<Instant>,
    is_initiating: bool,
    pending_peer: Option<(String, String)>,
    /// Virtual cursor position in **remote screen** coordinates.
    /// Used to detect when the cursor should return to the local machine.
    virtual_pos: (f64, f64),
    entry_side: Option<Side>,
    pending_transition_side: Option<Side>,
    /// The edge position (in remote screen coords) for a pending transition.
    pending_edge_pos: f64,
    last_transition_at: Instant,
    switch_tx: Option<Sender<ContinuityCmd>>,
}

impl ContinuityInner {
    fn new(data_tx: Sender<ContinuityData>) -> Self {
        Self {
            data_tx,
            data: ContinuityData::default(),
            last_message_at: None,
            is_initiating: false,
            pending_peer: None,
            virtual_pos: (0.0, 0.0),
            entry_side: None,
            pending_transition_side: None,
            pending_edge_pos: 0.0,
            last_transition_at: Instant::now() - Duration::from_secs(10),
            switch_tx: None,
        }
    }

    /// Returns the remote screen dimensions, falling back to local dims if unknown.
    fn remote_screen(&self) -> (i32, i32) {
        self.data.remote_screen.unwrap_or((self.data.screen_width, self.data.screen_height))
    }

    /// Initialize virtual_pos for sharing mode. The cursor enters the remote screen
    /// at `entry_side` with `edge_pos` along the entry edge (in remote coords).
    fn init_virtual_pos(&mut self, entry_side: Side, edge_pos: f64) {
        let (rw, rh) = self.remote_screen();
        let buffer = 40.0;
        self.virtual_pos = match entry_side {
            // Entered remote from the left edge → cursor starts near x=0
            Side::Right => (buffer, edge_pos.clamp(0.0, rh as f64)),
            // Entered remote from the right edge → cursor starts near x=remote_w
            Side::Left => (rw as f64 - buffer, edge_pos.clamp(0.0, rh as f64)),
            // Entered remote from the top edge → cursor starts near y=0
            Side::Bottom => (edge_pos.clamp(0.0, rw as f64), buffer),
            // Entered remote from the bottom edge → cursor starts near y=remote_h
            Side::Top => (edge_pos.clamp(0.0, rw as f64), rh as f64 - buffer),
        };
    }

    fn push(&self) {
        let _ = self.data_tx.try_send(self.data.clone());
    }

    async fn run(&mut self, cmd_rx: async_channel::Receiver<ContinuityCmd>, switch_rx: async_channel::Receiver<ContinuityCmd>) {
        use tokio::select;
        use tokio::time::{interval, Duration};

        let (discovery_tx, discovery_rx) = bounded::<DiscoveryEvent>(32);
        let (conn_tx, conn_rx) = bounded::<ConnectionEvent>(64);
        let (clipboard_tx, clipboard_rx) = bounded::<clipboard::ClipboardEvent>(32);
        let (input_tx, input_rx) = bounded::<input::InputEvent>(128);

        let mut discovery = discovery::AvahiDiscovery::new();
        let mut connection = connection::TcpConnectionProvider::new();
        let mut clipboard = clipboard::WaylandClipboard::new();
        let mut injection = input::WaylandInjection::new();
        let mut capture = input::EvdevCapture::new();
        let mut heartbeat = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));

        // Get initial screen dimensions from Niri (logical size of primary output).
        // The primary output is the one whose logical area contains (0,0) after
        // accounting for negative positions, or the largest output if none matches.
        if let Ok(mut sock) = niri_ipc::socket::Socket::connect() {
            if let Ok(Ok(niri_ipc::Response::Outputs(outputs))) = sock.send(niri_ipc::Request::Outputs) {
                let mut best: Option<(i32, i32)> = None;
                let mut best_area: u64 = 0;
                for output in outputs.values() {
                    if let Some(logical) = &output.logical {
                        let w = logical.width as i32;
                        let h = logical.height as i32;
                        // Prefer the output that covers (0,0), else the largest
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
                    info!("[continuity] detected primary output: {}x{} (logical)", w, h);
                    self.data.screen_width = w;
                    self.data.screen_height = h;
                }
            }
        }

        info!("[continuity] service started, device: {}", self.data.device_name);

        loop {
            let is_connected = self.data.active_connection.is_some();

            select! {
                Ok(cmd) = cmd_rx.recv() => {
                    self.handle_cmd(cmd, &mut discovery, &mut connection, &mut clipboard, &mut injection, &mut capture, &discovery_tx, &conn_tx, &clipboard_tx).await;
                }
                Ok(cmd) = switch_rx.recv() => {
                    // Internal commands routed from message handlers
                    self.handle_cmd(cmd, &mut discovery, &mut connection, &mut clipboard, &mut injection, &mut capture, &discovery_tx, &conn_tx, &clipboard_tx).await;
                }
                Ok(event) = discovery_rx.recv() => {
                    self.handle_discovery_event(event).await;
                }
                Ok(event) = conn_rx.recv() => {
                    self.handle_connection_event(event, &mut connection, &mut clipboard, &mut injection, &mut capture, &clipboard_tx, &input_tx).await;
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
            }
        }
    }

    async fn handle_cmd(
        &mut self,
        cmd: ContinuityCmd,
        discovery: &mut discovery::AvahiDiscovery,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
        discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
        clipboard_tx: &Sender<clipboard::ClipboardEvent>,
    ) {
        match cmd {
            ContinuityCmd::SetEnabled(on) => {
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
                    self.data.sharing_mode = SharingMode::Idle;
                    self.data.pending_pin = None;
                    self.entry_side = None;
                    self.last_message_at = None;
                }
                self.push();
            }
            ContinuityCmd::StartDiscovery => {
                if self.data.enabled {
                    info!("[continuity] starting peer discovery");
                    if let Err(e) = discovery.browse(discovery_tx.clone()) {
                        error!("[continuity] discovery browse failed: {e}");
                    }
                }
            }
            ContinuityCmd::StopDiscovery => {
                discovery.stop_browse();
                self.data.peers.clear();
                self.push();
            }
            ContinuityCmd::ConnectToPeer(peer_id) => {
                if let Some(peer) = self.data.peers.iter().find(|p| p.device_id == peer_id) {
                    let name = peer.device_name.clone();
                    let addr_v4 = peer.address;
                    let addr_v6 = peer.address_v6;
                    info!("[continuity] connecting to {name}");
                    self.is_initiating = true;
                    connection.connect_dual(
                        addr_v4,
                        addr_v6,
                        conn_tx.clone(),
                        self.data.device_id.clone(),
                        self.data.device_name.clone(),
                    );
                }
            }
            ContinuityCmd::ConfirmPin => {
                if let Some(pending) = self.data.pending_pin.take() {
                    info!("[continuity] PIN confirmed locally");
                    connection.send_message(protocol::Message::PinConfirm { pin: pending.pin });
                    if pending.is_incoming {
                        // We received the pin and accepted it. We can now consider the connection active.
                        info!("[continuity] Connection to {} is now active", pending.peer_name);
                        self.data.active_connection = Some(ActiveConnectionInfo {
                            peer_id: pending.peer_id,
                            peer_name: pending.peer_name,
                            since: Instant::now(),
                        });
                        self.last_message_at = Some(Instant::now());

                        // Exchange screen info
                        connection.send_message(protocol::Message::ScreenInfo {
                            width: self.data.screen_width,
                            height: self.data.screen_height,
                        });

                        // Start clipboard sync
                        if let Err(e) = clipboard.start_monitoring(clipboard_tx.clone()) {
                            error!("[continuity] failed to start clipboard monitoring: {e}");
                        }
                        
                        // Start injection
                        if let Err(e) = injection.start() {
                            error!("[continuity] failed to start input injection: {e}");
                        }
                    }
                }
                self.push();
            }
            ContinuityCmd::RejectPin => {
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
                self.data.sharing_mode = SharingMode::Idle;
                self.push();
            }
            ContinuityCmd::Disconnect => {
                info!("[continuity] disconnecting");
                connection.disconnect_active();
                clipboard.stop_monitoring();
                injection.stop();
                capture.stop();
                self.data.active_connection = None;
                self.data.sharing_mode = SharingMode::Idle;
                self.last_message_at = None;
                self.push();
            }
            ContinuityCmd::ForceLocal => {
                if self.data.sharing_mode == SharingMode::Sharing
                    || self.data.sharing_mode == SharingMode::Pending
                    || self.data.sharing_mode == SharingMode::PendingSwitch
                    || self.data.sharing_mode == SharingMode::Receiving
                {
                    info!("[continuity] forcing cursor back to local");
                    self.data.sharing_mode = SharingMode::Idle;
                    self.pending_transition_side = None;
                    capture.stop();
                    connection.send_message(protocol::Message::TransitionCancel);
                    self.push();
                }
            }
            ContinuityCmd::StartSharing(side, local_edge_pos) => {
                if self.data.active_connection.is_some() && self.data.sharing_mode == SharingMode::Idle {
                    // 2 second cooldown to prevent jitter
                    if self.last_transition_at.elapsed() < Duration::from_secs(2) {
                        return;
                    }

                    // Map local edge position to remote screen coordinates
                    let arrangement = self.data.active_peer_config().arrangement;
                    let remote_edge_pos = arrangement.local_to_remote_edge(local_edge_pos);
                    info!("[continuity] initiating sharing via {:?}, local_pos={:.0} remote_pos={:.0}", side, local_edge_pos, remote_edge_pos);
                    self.data.sharing_mode = SharingMode::Pending;
                    self.pending_transition_side = Some(side);
                    self.pending_edge_pos = remote_edge_pos;
                    self.last_transition_at = Instant::now();

                    connection.send_message(protocol::Message::EdgeTransition { side, edge_pos: remote_edge_pos });
                    self.push();
                }
            }
            ContinuityCmd::StopSharing(edge_pos) => {
                if self.data.sharing_mode == SharingMode::Sharing || self.data.sharing_mode == SharingMode::Pending {
                    info!("[continuity] stopping sharing");
                    self.data.sharing_mode = SharingMode::Idle;
                    self.pending_transition_side = None;
                    capture.stop();
                    connection.send_message(protocol::Message::TransitionCancel);
                    self.push();
                } else if self.data.sharing_mode == SharingMode::Receiving {
                    // User wants to take over cursor from Receiving mode.
                    // The edge is on our active_peer_config side (where the peer is).
                    let side = self.data.active_peer_config().arrangement.side;
                    info!("[continuity] requesting switch back to sharing via {:?}, edge_pos={:.0}", side, edge_pos);
                    self.data.sharing_mode = SharingMode::PendingSwitch;
                    connection.send_message(protocol::Message::SwitchTransition { side, edge_pos });
                    self.push();
                }
            }
            ContinuityCmd::SendInput(msg) => {
                if self.data.sharing_mode == SharingMode::Sharing {
                    connection.send_message(msg);
                }
            }
            ContinuityCmd::SetPeerArrangement(arrangement) => {
                if let Some(conn) = &self.data.active_connection {
                    let peer_id = conn.peer_id.clone();
                    let config = self.data.peer_configs.entry(peer_id).or_default();
                    config.arrangement = arrangement;
                    config.version += 1;
                    let version = config.version;
                    
                    info!("[continuity] updated config for peer {}: {:?} (v{})", conn.peer_name, arrangement, version);
                    
                    // Send sync message to peer
                    connection.send_message(protocol::Message::ConfigSync {
                        arrangement: arrangement.side,
                        offset: arrangement.offset,
                        version,
                    });
                }
                self.push();
            }
            ContinuityCmd::SetScreenSize(w, h) => {
                info!("[continuity] screen size set to {}x{}", w, h);
                self.data.screen_width = w;
                self.data.screen_height = h;
                self.push();
            }
            ContinuityCmd::SwitchToReceiving(side) => {
                // Internal: called from SwitchTransition message handler via switch channel.
                // We (the old sharer) know our virtual_pos which approximates the cursor
                // position on the remote screen. Send it so the new sharer can init correctly.
                if self.data.sharing_mode == SharingMode::Sharing || self.data.sharing_mode == SharingMode::PendingSwitch {
                    // Extract the edge-parallel component of virtual_pos (in remote coords)
                    let edge_pos = match side {
                        Side::Left | Side::Right => self.virtual_pos.1,
                        Side::Top | Side::Bottom => self.virtual_pos.0,
                    };
                    info!("[continuity] switching to Receiving via {:?}, edge_pos={:.0}", side, edge_pos);
                    capture.stop();

                    self.data.sharing_mode = SharingMode::Receiving;
                    self.entry_side = None;
                    self.pending_transition_side = None;

                    // Start injection for receiving input from peer
                    use input::InputInjection;
                    if let Err(e) = injection.start() {
                        error!("[continuity] failed to start injection for switch: {e}");
                    }

                    // Warp the local physical cursor to where it "re-enters" our screen.
                    if let Err(e) = injection.warp(side, edge_pos, self.data.screen_width, self.data.screen_height) {
                        error!("[continuity] failed to warp cursor for switch: {e}");
                    }

                    connection.send_message(protocol::Message::SwitchConfirm { side, edge_pos });
                    self.push();
                }
            }
        }
    }

    fn handle_heartbeat(&mut self, connection: &mut connection::TcpConnectionProvider, capture: &mut input::EvdevCapture) {
        if let Some(last) = self.last_message_at {
            // Use much shorter timeout if we are currently being controlled (Receiving),
            // so the user isn't "trapped" if the other side crashes.
            let timeout = if self.data.sharing_mode == SharingMode::Receiving {
                Duration::from_secs(5)
            } else if self.data.sharing_mode == SharingMode::Pending || self.data.sharing_mode == SharingMode::PendingSwitch {
                // Short timeout for pending transitions (don't wait forever for ack)
                Duration::from_secs(5)
            } else {
                Duration::from_secs(CONNECTION_TIMEOUT_SECS)
            };

            if last.elapsed() > timeout {
                warn!("[continuity] peer timed out (no message for {:?})", timeout);
                connection.disconnect_active();
                capture.stop();
                self.data.active_connection = None;
                self.data.sharing_mode = SharingMode::Idle;
                self.last_message_at = None;
                self.push();
                return;
            }
        }

        // Send heartbeat
        connection.send_message(protocol::Message::Heartbeat);
    }

    async fn handle_discovery_event(&mut self, event: DiscoveryEvent) {
        match event {
            DiscoveryEvent::PeerFound(peer) => {
                // Skip ourselves (match by hostname) and duplicates
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
                    self.data.sharing_mode = SharingMode::Idle;
                    self.entry_side = None;
                    self.last_message_at = None;
                }
                self.push();
            }
        }
    }

    async fn handle_connection_event(
        &mut self,
        event: ConnectionEvent,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
        clipboard_tx: &Sender<clipboard::ClipboardEvent>,
        input_tx: &Sender<input::InputEvent>,
    ) {
        match event {
            ConnectionEvent::IncomingConnection { addr, write_tx } => {
                info!("[continuity] incoming connection from {addr}");
                self.is_initiating = false;
                // Store the write channel so we can send messages back
                connection.set_active_write(write_tx);

                // Send our Hello as server response
                let hello = protocol::Message::Hello {
                    device_id: self.data.device_id.clone(),
                    device_name: self.data.device_name.clone(),
                    version: protocol::PROTOCOL_VERSION,
                };
                connection.send_message(hello);
            }
            ConnectionEvent::HandshakeComplete { .. } => {
                // Not used much with the new flow, handled inside MessageReceived
            }
            ConnectionEvent::Disconnected { reason } => {
                info!("[continuity] disconnected: {reason}");
                self.data.active_connection = None;
                self.data.sharing_mode = SharingMode::Idle;
                self.data.pending_pin = None;
                self.data.remote_screen = None;
                self.pending_peer = None;
                self.entry_side = None;
                self.last_message_at = None;
                clipboard.stop_monitoring();
                injection.stop();
                capture.stop();
                self.push();
            }
            ConnectionEvent::MessageReceived(msg) => {
                self.last_message_at = Some(Instant::now());
                match msg {
                    protocol::Message::Hello { device_id, device_name, version } => {
                        if version != protocol::PROTOCOL_VERSION {
                            warn!("[continuity] peer version mismatch: {version}");
                            connection.disconnect_active();
                            self.last_message_at = None;
                            return;
                        }
                        info!("[continuity] handshake from {device_name} ({device_id})");
                        
                        self.pending_peer = Some((device_id.clone(), device_name.clone()));
                        
                        if self.is_initiating {
                            let pin = format!("{:06}", uuid::Uuid::new_v4().as_u128() % 1000000);
                            info!("[continuity] initiating pairing, generating PIN: {pin}");
                            self.data.pending_pin = Some(PendingPin {
                                pin: pin.clone(),
                                peer_id: device_id,
                                peer_name: device_name,
                                is_incoming: false,
                            });
                            connection.send_message(protocol::Message::PinRequest { pin });
                            self.push();
                        }
                    }
                    protocol::Message::PinRequest { pin } => {
                        if let Some((peer_id, peer_name)) = self.pending_peer.clone() {
                            info!("[continuity] received pairing request with PIN: {pin}");
                            self.data.pending_pin = Some(PendingPin {
                                pin,
                                peer_id,
                                peer_name,
                                is_incoming: true,
                            });
                            self.push();
                        }
                    }
                    protocol::Message::PinConfirm { pin } => {
                        if let Some(pending) = &self.data.pending_pin {
                            if pending.pin == pin {
                                info!("[continuity] peer confirmed PIN, connection active");
                                self.data.active_connection = Some(ActiveConnectionInfo {
                                    peer_id: pending.peer_id.clone(),
                                    peer_name: pending.peer_name.clone(),
                                    since: Instant::now(),
                                });
                                self.data.pending_pin = None;
                                self.push();

                                // Exchange screen info
                                connection.send_message(protocol::Message::ScreenInfo {
                                    width: self.data.screen_width,
                                    height: self.data.screen_height,
                                });

                                // Start clipboard sync
                                if let Err(e) = clipboard.start_monitoring(clipboard_tx.clone()) {
                                    error!("[continuity] failed to start clipboard monitoring: {e}");
                                }

                                // Start injection
                                use input::InputInjection;
                                if let Err(e) = injection.start() {
                                    error!("[continuity] failed to start input injection: {e}");
                                }
                            } else {
                                warn!("[continuity] peer sent incorrect PIN confirmation");
                                connection.disconnect_active();
                            }
                        }
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
                        use input::InputInjection;
                        let _ = injection.inject(&msg);
                    }
                    protocol::Message::ScreenInfo { width, height } => {
                        info!("[continuity] peer screen: {}x{}", width, height);
                        self.data.remote_screen = Some((width, height));
                        self.push();
                        
                        // After exchanging screen info, also exchange current config
                        let config = self.data.active_peer_config();
                        connection.send_message(protocol::Message::ConfigSync {
                            arrangement: config.arrangement.side,
                            offset: config.arrangement.offset,
                            version: config.version,
                        });
                        
                        self.push();
                    }
                    protocol::Message::ConfigSync { arrangement, offset, version } => {
                        if let Some(conn) = &self.data.active_connection {
                            let peer_id = conn.peer_id.clone();
                            let config = self.data.peer_configs.entry(peer_id).or_default();
                            
                            if version > config.version {
                                info!("[continuity] adopting newer config from peer (v{} > v{}): {:?} offset {}", 
                                    version, config.version, arrangement, offset);
                                
                                // The peer says "you are to my <arrangement>".
                                // So the peer is to our <opposite>.
                                // Negate the offset: if the peer is N px below their
                                // top edge, we are N px above ours.
                                config.arrangement = PeerArrangement {
                                    side: arrangement.opposite(),
                                    offset: -offset,
                                };
                                config.version = version;
                                self.push();
                            } else {
                                info!("[continuity] ignoring older/same config from peer (v{} <= v{})", version, config.version);
                            }
                        }
                    }
                    protocol::Message::EdgeTransition { side, edge_pos } => {
                        if self.data.sharing_mode == SharingMode::Idle {
                            let arrangement = self.data.active_peer_config().arrangement;
                            // Map the peer's exit position to our local entry position
                            let mapped_pos = arrangement.remote_to_local_edge(edge_pos);
                            let local_side = side.opposite();

                            info!("[continuity] accepting sharing from peer: peer_exit={:?}@{} -> local_entry={:?}@{}", 
                                side, edge_pos, local_side, mapped_pos);

                            self.data.sharing_mode = SharingMode::Receiving;

                            // Physical cursor positioning
                            if let Err(e) = injection.warp(local_side, mapped_pos, self.data.screen_width, self.data.screen_height) {
                                error!("[continuity] failed to warp cursor: {e}");
                            }

                            connection.send_message(protocol::Message::TransitionAck { accepted: true });
                            self.push();
                        } else {
                            info!("[continuity] rejecting sharing from peer (already in {:?})", self.data.sharing_mode);
                            connection.send_message(protocol::Message::TransitionAck { accepted: false });
                        }
                    }
                    protocol::Message::TransitionAck { accepted } => {
                        if self.data.sharing_mode == SharingMode::Pending {
                            if accepted {
                                let side = self.pending_transition_side.unwrap_or(Side::Right);
                                let edge_pos = self.pending_edge_pos;
                                info!("[continuity] transition accepted, sharing via {:?}, edge_pos={:.0}", side, edge_pos);
                                self.data.sharing_mode = SharingMode::Sharing;
                                self.entry_side = Some(side);
                                self.pending_transition_side = None;

                                // Initialize virtual_pos in remote screen coordinates
                                self.init_virtual_pos(side, edge_pos);
                                info!("[continuity] virtual_pos initialized to ({:.0}, {:.0})", self.virtual_pos.0, self.virtual_pos.1);

                                if let Err(e) = capture.start(input_tx.clone()) {
                                    error!("[continuity] failed to start input capture: {e}");
                                    self.data.sharing_mode = SharingMode::Idle;
                                    self.entry_side = None;
                                    connection.send_message(protocol::Message::TransitionCancel);
                                }
                            } else {
                                info!("[continuity] transition rejected by peer");
                                self.data.sharing_mode = SharingMode::Idle;
                                self.pending_transition_side = None;
                            }
                            self.push();
                        }
                    }
                    protocol::Message::TransitionCancel => {
                        info!("[continuity] forcing cursor back to local");
                        self.data.sharing_mode = SharingMode::Idle;
                        self.pending_transition_side = None;
                        self.push();
                    }
                    protocol::Message::SwitchTransition { side, edge_pos: _ } => {
                        // Peer wants to take over (switch from Sharing → Receiving on our side)
                        if self.data.sharing_mode == SharingMode::Sharing {
                            info!("[continuity] peer requesting switch via {:?}", side);
                            self.data.sharing_mode = SharingMode::PendingSwitch;
                            self.push();
                            // We need capture to stop, route through cmd
                            let _ = self.switch_tx.as_ref().unwrap().try_send(ContinuityCmd::SwitchToReceiving(side));
                        } else {
                            info!("[continuity] rejecting switch (not in Sharing, currently {:?})", self.data.sharing_mode);
                            // Send a dummy SwitchConfirm to unblock the peer
                            connection.send_message(protocol::Message::SwitchConfirm { side, edge_pos: 0.0 });
                        }
                    }
                    protocol::Message::SwitchConfirm { side, edge_pos } => {
                        if self.data.sharing_mode == SharingMode::PendingSwitch {
                            info!("[continuity] switch confirmed, taking over as Sharer via {:?}, edge_pos={:.0}", side, edge_pos);

                            self.data.sharing_mode = SharingMode::Sharing;
                            self.entry_side = Some(side);

                            // The old sharer sent us their virtual_pos along the edge (in remote coords).
                            // Map it to our coordinate system for init.
                            let arrangement = self.data.active_peer_config().arrangement;
                            let mapped_pos = arrangement.local_to_remote_edge(edge_pos);
                            self.init_virtual_pos(side, mapped_pos);
                            info!("[continuity] virtual_pos initialized to ({:.0}, {:.0})", self.virtual_pos.0, self.virtual_pos.1);

                            if let Err(e) = capture.start(input_tx.clone()) {
                                error!("[continuity] failed to start input capture after switch: {e}");
                                self.data.sharing_mode = SharingMode::Idle;
                                self.entry_side = None;
                                connection.send_message(protocol::Message::TransitionCancel);
                            }
                            self.push();
                        }
                    }
                    protocol::Message::Connected => {
                        info!("[continuity] connection established");
                    }
                    protocol::Message::Heartbeat => {}
                    protocol::Message::Disconnect { reason } => {
                        info!("[continuity] peer disconnected: {reason}");
                        self.data.active_connection = None;
                        self.data.sharing_mode = SharingMode::Idle;
                        self.data.pending_pin = None;
                        self.pending_peer = None;
                        self.last_message_at = None;
                        clipboard.stop_monitoring();
                        injection.stop();
                        capture.stop();
                        self.push();
                    }
                }
            }
            ConnectionEvent::Error(e) => {
                error!("[continuity] connection error: {e}");
            }
        }
    }

    async fn handle_clipboard_event(
        &mut self,
        event: clipboard::ClipboardEvent,
        connection: &connection::TcpConnectionProvider,
    ) {
        match event {
            clipboard::ClipboardEvent::ContentChanged { content, mime_type } => {
                if self.data.active_connection.is_some() {
                    info!("[continuity] clipboard changed, sending to peer");
                    connection.send_message(protocol::Message::ClipboardUpdate {
                        content,
                        mime_type,
                    });
                }
            }
        }
    }

    async fn handle_input_capture_event(
        &mut self,
        event: input::InputEvent,
        connection: &connection::TcpConnectionProvider,
        capture: &mut input::EvdevCapture,
    ) {
        if self.data.sharing_mode == SharingMode::Sharing {
            match event {
                input::InputEvent::CursorMove { dx, dy } => {
                    let (rw, rh) = self.remote_screen();
                    let rw = rw as f64;
                    let rh = rh as f64;

                    // Update virtual position (tracked in remote screen coordinates)
                    self.virtual_pos.0 += dx;
                    self.virtual_pos.1 += dy;

                    // Clamp with 100px overflow buffer for edge detection
                    self.virtual_pos.0 = self.virtual_pos.0.clamp(-100.0, rw + 100.0);
                    self.virtual_pos.1 = self.virtual_pos.1.clamp(-100.0, rh + 100.0);

                    // Check for return transition: cursor moved back past the entry edge
                    if let Some(entry) = self.entry_side {
                        let should_return = match entry {
                            Side::Left if self.virtual_pos.0 > rw => true,
                            Side::Right if self.virtual_pos.0 < 0.0 => true,
                            Side::Top if self.virtual_pos.1 > rh => true,
                            Side::Bottom if self.virtual_pos.1 < 0.0 => true,
                            _ => false,
                        };

                        if should_return {
                            info!("[continuity] return transition at vpos=({:.0},{:.0})", self.virtual_pos.0, self.virtual_pos.1);
                            self.data.sharing_mode = SharingMode::Idle;
                            self.entry_side = None;
                            self.last_transition_at = Instant::now();
                            capture.stop();
                            connection.send_message(protocol::Message::TransitionCancel);
                            self.push();
                            return;
                        }
                    }

                    connection.send_message(protocol::Message::CursorMove { dx, dy });
                }
                input::InputEvent::KeyPress { key, state } => {
                    connection.send_message(protocol::Message::KeyPress { key, state });
                }
                input::InputEvent::KeyRelease { key } => {
                    connection.send_message(protocol::Message::KeyRelease { key });
                }
                input::InputEvent::PointerButton { button, state } => {
                    connection.send_message(protocol::Message::PointerButton { button, state });
                }
                input::InputEvent::PointerAxis { dx, dy } => {
                    connection.send_message(protocol::Message::PointerAxis { dx, dy });
                }
                input::InputEvent::EmergencyExit => {
                    info!("[continuity] kernel emergency exit requested");
                    self.data.sharing_mode = SharingMode::Idle;
                    capture.stop();
                    connection.send_message(protocol::Message::TransitionCancel);
                    self.push();
                }
            };
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn config_dir() -> std::path::PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            std::path::PathBuf::from(home).join(".config")
        });
    base.join("axis")
}

fn persistent_device_id() -> String {
    let path = config_dir().join("continuity_id");
    if let Ok(id) = std::fs::read_to_string(&path) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &id);
    id
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "axis-device".into())
}
