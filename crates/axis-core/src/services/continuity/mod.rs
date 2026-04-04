pub mod clipboard;
pub mod connection;
pub mod dbus;
pub mod discovery;
pub mod input;
pub mod known_peers;
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
const RECONNECT_MAX_ATTEMPTS: u32 = 5;
const RECONNECT_BASE_DELAY_MS: u64 = 1000;
const PIN_EXPIRY_SECS: u64 = 30;

// ── Data Types ─────────────────────────────────────────────────────────

/// State machine for cursor sharing transitions.
///
/// Each variant carries only the data that is valid in that state,
/// making impossible states unrepresentable at compile time.
#[derive(Debug, Clone, PartialEq)]
pub enum SharingState {
    /// No sharing active — cursor is local.
    Idle,
    /// Edge transition requested, waiting for peer to accept.
    Pending {
        entry_side: Side,
        edge_pos: f64,
    },
    /// Actively sharing — we control the remote cursor.
    Sharing {
        entry_side: Side,
        virtual_pos: (f64, f64),
    },
    /// Peer controls our cursor — we receive input.
    Receiving,
    /// Peer requested to switch roles — waiting for confirmation.
    PendingSwitch,
}

impl SharingState {
    pub fn is_idle(&self) -> bool { matches!(self, Self::Idle) }
    pub fn is_active(&self) -> bool { !self.is_idle() }
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
    pub created_at: Instant,
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
    pub trusted: bool,
    pub arrangement: PeerArrangement,
    pub clipboard: bool,
    pub audio: bool,
    pub drag_drop: bool,
    pub version: u64,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            trusted: false,
            arrangement: PeerArrangement::default(),
            clipboard: true,
            audio: false,
            drag_drop: false,
            version: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconnectState {
    pub peer_id: String,
    pub peer_name: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContinuityData {
    pub device_id: String,
    pub device_name: String,
    pub enabled: bool,
    pub peers: Vec<PeerInfo>,
    pub active_connection: Option<ActiveConnectionInfo>,
    pub sharing_state: SharingState,
    pub pending_pin: Option<PendingPin>,
    pub peer_configs: std::collections::HashMap<String, PeerConfig>,
    pub screen_width: i32,
    pub screen_height: i32,
    pub remote_screen: Option<(i32, i32)>,
    pub reconnect: Option<ReconnectState>,
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
            sharing_state: SharingState::Idle,
            pending_pin: None,
            peer_configs: std::collections::HashMap::new(),
            screen_width: 1920,
            screen_height: 1080,
            remote_screen: None,
            reconnect: None,
        }
    }
}

// ── Commands ───────────────────────────────────────────────────────────

pub enum ContinuityCmd {
    SetEnabled(bool),
    ConnectToPeer(String),
    Unpair(String),
    ConfirmPin,
    RejectPin,
    Disconnect,
    CancelReconnect,
    ForceLocal,
    StartDiscovery,
    StopDiscovery,
    StartSharing(Side, f64),
    StopSharing(f64),
    SendInput(protocol::Message),
    SetPeerArrangement(PeerArrangement),
    UpdatePeerConfigs(std::collections::HashMap<String, PeerConfig>),
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
    last_transition_at: Instant,
    switch_tx: Option<Sender<ContinuityCmd>>,
    known_peers: known_peers::KnownPeersStore,
}

impl ContinuityInner {
    fn new(data_tx: Sender<ContinuityData>) -> Self {
        let known_peers = known_peers::load_known_peers();
        let mut data = ContinuityData::default();
        
        for (id, peer) in &known_peers.peers {
            if peer.trusted {
                data.peer_configs.insert(id.clone(), PeerConfig {
                    trusted: true,
                    ..Default::default()
                });
            }
        }
        
        Self {
            data_tx,
            data,
            last_message_at: None,
            is_initiating: false,
            pending_peer: None,
            last_transition_at: Instant::now() - Duration::from_secs(10),
            switch_tx: None,
            known_peers,
        }
    }
    
    fn persist_known_peers(&self) {
        let known_map: std::collections::HashMap<_, _> = self.known_peers.peers.iter()
            .map(|(id, p)| (id, p))
            .collect();
        let peers_map: std::collections::HashMap<_, _> = self.data.peers.iter()
            .map(|p| (&p.device_id, p))
            .collect();

        let peers = self.data.peer_configs.iter()
            .filter(|(_, c)| c.trusted)
            .filter_map(|(id, config)| {
                let known_peer = known_map.get(id).map(|p| {
                    (id.clone(), known_peers::KnownPeer {
                        device_id: p.device_id.clone(),
                        device_name: p.device_name.clone(),
                        hostname: p.hostname.clone(),
                        address: p.address.clone(),
                        address_v6: p.address_v6.clone(),
                        trusted: config.trusted,
                    })
                }).or_else(|| {
                    peers_map.get(id).map(|peer| {
                        (id.clone(), known_peers::KnownPeer {
                            device_id: peer.device_id.clone(),
                            device_name: peer.device_name.clone(),
                            hostname: peer.hostname.clone(),
                            address: peer.address.to_string(),
                            address_v6: peer.address_v6.map(|a| a.to_string()),
                            trusted: config.trusted,
                        })
                    })
                });
                known_peer
            })
            .collect();

        known_peers::save_known_peers(&known_peers::KnownPeersStore { peers });
    }

    /// Returns the remote screen dimensions, falling back to local dims if unknown.
    fn remote_screen(&self) -> (i32, i32) {
        self.data.remote_screen.unwrap_or((self.data.screen_width, self.data.screen_height))
    }

    /// Initialize virtual_pos for sharing mode. The cursor enters the remote screen
    /// at `entry_side` with `edge_pos` along the entry edge (in remote coords).
    fn init_virtual_pos(entry_side: Side, edge_pos: f64, remote_w: i32, remote_h: i32) -> (f64, f64) {
        let (rw, rh) = (remote_w as f64, remote_h as f64);
        let buffer = 40.0;
        match entry_side {
            Side::Right => (buffer, edge_pos.clamp(0.0, rh)),
            Side::Left => (rw - buffer, edge_pos.clamp(0.0, rh)),
            Side::Bottom => (edge_pos.clamp(0.0, rw), buffer),
            Side::Top => (edge_pos.clamp(0.0, rw), rh - buffer),
        }
    }

    fn push(&self) {
        let _ = self.data_tx.try_send(self.data.clone());
    }

    fn start_reconnect(&mut self, peer_id: &str, peer_name: &str) {
        if self.data.reconnect.is_some() {
            return;
        }
        let attempt = 1;
        let delay_secs = RECONNECT_BASE_DELAY_MS / 1000;
        info!("[continuity] scheduling reconnect for {peer_name} (attempt {}/{}, in {}s)",
            attempt, RECONNECT_MAX_ATTEMPTS, delay_secs);
        self.data.reconnect = Some(ReconnectState {
            peer_id: peer_id.to_string(),
            peer_name: peer_name.to_string(),
            attempt,
            max_attempts: RECONNECT_MAX_ATTEMPTS,
            delay_secs,
        });
    }

    fn cancel_reconnect(&mut self) {
        self.data.reconnect = None;
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
        let mut reconnect_timer = interval(Duration::from_secs(1));
        reconnect_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

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
                _ = reconnect_timer.tick() => {
                    self.handle_reconnect_tick(&mut connection, &conn_tx).await;
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
            ContinuityCmd::SetEnabled(on) => self.handle_set_enabled(on, discovery, connection, clipboard, injection, capture, discovery_tx, conn_tx).await,
            ContinuityCmd::StartDiscovery => self.handle_start_discovery(discovery, discovery_tx).await,
            ContinuityCmd::StopDiscovery => self.handle_stop_discovery(discovery).await,
            ContinuityCmd::ConnectToPeer(peer_id) => self.handle_connect_to_peer(&peer_id, connection, discovery_tx, conn_tx).await,
            ContinuityCmd::ConfirmPin => self.handle_confirm_pin(connection, clipboard, injection, capture, clipboard_tx).await,
            ContinuityCmd::RejectPin => self.handle_reject_pin(connection, clipboard, injection, capture).await,
            ContinuityCmd::Disconnect => self.handle_disconnect(connection, clipboard, injection, capture).await,
            ContinuityCmd::CancelReconnect => self.handle_cancel_reconnect().await,
            ContinuityCmd::Unpair(peer_id) => self.handle_unpair(&peer_id, connection, clipboard, injection, capture).await,
            ContinuityCmd::ForceLocal => self.handle_force_local(capture, connection).await,
            ContinuityCmd::StartSharing(side, local_edge_pos) => self.handle_start_sharing(side, local_edge_pos, connection).await,
            ContinuityCmd::StopSharing(edge_pos) => self.handle_stop_sharing(edge_pos, connection, capture).await,
            ContinuityCmd::SendInput(msg) => self.handle_send_input(&msg, connection).await,
            ContinuityCmd::SetPeerArrangement(arrangement) => self.handle_set_peer_arrangement(arrangement, connection).await,
            ContinuityCmd::UpdatePeerConfigs(configs) => self.handle_update_peer_configs(configs).await,
            ContinuityCmd::SetScreenSize(w, h) => self.handle_set_screen_size(w, h).await,
            ContinuityCmd::SwitchToReceiving(side) => self.handle_switch_to_receiving(side, connection, injection).await,
        }
    }

    async fn handle_set_enabled(
        &mut self,
        on: bool,
        discovery: &mut discovery::AvahiDiscovery,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
        discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
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

    async fn handle_start_discovery(
        &mut self,
        discovery: &mut discovery::AvahiDiscovery,
        discovery_tx: &Sender<DiscoveryEvent>,
    ) {
        if self.data.enabled {
            info!("[continuity] starting peer discovery");
            if let Err(e) = discovery.browse(discovery_tx.clone()) {
                error!("[continuity] discovery browse failed: {e}");
            }
        }
    }

    async fn handle_stop_discovery(&mut self, discovery: &mut discovery::AvahiDiscovery) {
        discovery.stop_browse();
        self.data.peers.clear();
        self.push();
    }

    async fn handle_connect_to_peer(
        &mut self,
        peer_id: &str,
        connection: &mut connection::TcpConnectionProvider,
        _discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
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

    async fn handle_confirm_pin(
        &mut self,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        _capture: &mut input::EvdevCapture,
        clipboard_tx: &Sender<clipboard::ClipboardEvent>,
    ) {
        if let Some(pending) = self.data.pending_pin.take() {
            info!("[continuity] PIN confirmed locally");
            connection.send_message(protocol::Message::PinConfirm { pin: pending.pin.clone() });
            
            self.data.peer_configs.entry(pending.peer_id.clone()).or_default().trusted = true;
            self.persist_known_peers();
            
            if pending.is_incoming {
                info!("[continuity] Connection to {} is now active", pending.peer_name);
                self.data.active_connection = Some(ActiveConnectionInfo {
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

    async fn handle_reject_pin(
        &mut self,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
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

    async fn handle_disconnect(
        &mut self,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
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

    async fn handle_cancel_reconnect(&mut self) {
        if self.data.reconnect.is_some() {
            info!("[continuity] reconnect cancelled");
            self.data.reconnect = None;
            self.push();
        }
    }

    async fn handle_unpair(
        &mut self,
        peer_id: &str,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
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

    async fn handle_force_local(
        &mut self,
        capture: &mut input::EvdevCapture,
        connection: &mut connection::TcpConnectionProvider,
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

    async fn handle_start_sharing(
        &mut self,
        side: Side,
        local_edge_pos: f64,
        connection: &mut connection::TcpConnectionProvider,
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

    async fn handle_stop_sharing(
        &mut self,
        edge_pos: f64,
        connection: &mut connection::TcpConnectionProvider,
        capture: &mut input::EvdevCapture,
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

    async fn handle_send_input(
        &mut self,
        msg: &protocol::Message,
        connection: &mut connection::TcpConnectionProvider,
    ) {
        if matches!(self.data.sharing_state, SharingState::Sharing { .. }) {
            connection.send_message(msg.clone());
        }
    }

    async fn handle_set_peer_arrangement(
        &mut self,
        arrangement: PeerArrangement,
        connection: &mut connection::TcpConnectionProvider,
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

    async fn handle_update_peer_configs(
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

    async fn handle_set_screen_size(&mut self, w: i32, h: i32) {
        info!("[continuity] screen size set to {}x{}", w, h);
        self.data.screen_width = w;
        self.data.screen_height = h;
        self.push();
    }

    async fn handle_switch_to_receiving(
        &mut self,
        side: Side,
        connection: &mut connection::TcpConnectionProvider,
        injection: &mut input::WaylandInjection,
    ) {
        let virtual_pos = match &self.data.sharing_state {
            SharingState::Sharing { virtual_pos, .. } => Some(*virtual_pos),
            SharingState::PendingSwitch => None,
            _ => None,
        };
        if virtual_pos.is_some() || matches!(self.data.sharing_state, SharingState::PendingSwitch) {
            let edge_pos = match side {
                Side::Left | Side::Right => virtual_pos.map(|v| v.1).unwrap_or(0.0),
                Side::Top | Side::Bottom => virtual_pos.map(|v| v.0).unwrap_or(0.0),
            };
            info!("[continuity] switching to Receiving via {:?}, edge_pos={:.0}", side, edge_pos);
            use input::InputInjection;

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

    fn handle_heartbeat(&mut self, connection: &mut connection::TcpConnectionProvider, capture: &mut input::EvdevCapture) {
        if let Some(last) = self.last_message_at {
            // Use much shorter timeout if we are currently being controlled (Receiving),
            // so the user isn't "trapped" if the other side crashes.
            let timeout = if matches!(self.data.sharing_state, SharingState::Receiving) {
                Duration::from_secs(5)
            } else if matches!(&self.data.sharing_state, SharingState::Pending { .. } | SharingState::PendingSwitch) {
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
                self.data.sharing_state = SharingState::Idle;
                self.last_message_at = None;
                self.push();
                return;
            }
        }

        // Send heartbeat
        connection.send_message(protocol::Message::Heartbeat);

        // Check PIN expiry
        if let Some(pin) = &self.data.pending_pin {
            if pin.created_at.elapsed() > Duration::from_secs(PIN_EXPIRY_SECS) {
                warn!("[continuity] PIN expired ({}s timeout)", PIN_EXPIRY_SECS);
                self.data.pending_pin = None;
                connection.send_message(protocol::Message::Disconnect {
                    reason: "PIN expired".to_string(),
                });
                connection.disconnect_active();
                self.push();
            }
        }
    }

    async fn handle_reconnect_tick(
        &mut self,
        connection: &mut connection::TcpConnectionProvider,
        conn_tx: &Sender<ConnectionEvent>,
    ) {
        let reconnect = match &self.data.reconnect {
            Some(r) => r.clone(),
            None => return,
        };

        if reconnect.delay_secs > 0 {
            self.data.reconnect.as_mut().unwrap().delay_secs -= 1;
            return;
        }

        if reconnect.attempt > reconnect.max_attempts {
            warn!("[continuity] reconnect failed after {} attempts, giving up", reconnect.attempt - 1);
            self.data.reconnect = None;
            self.push();
            return;
        }

        info!("[continuity] reconnect attempt {}/{} for {}",
            reconnect.attempt, reconnect.max_attempts, reconnect.peer_name);

        // Find peer info from discovered peers or known peers
        let peer_info = self.data.peers.iter()
            .find(|p| p.device_id == reconnect.peer_id)
            .cloned();

        if let Some(peer) = peer_info {
            self.is_initiating = true;
            self.pending_peer = Some((peer.device_id.clone(), peer.device_name.clone()));

            connection.connect_dual(
                peer.address,
                peer.address_v6,
                conn_tx.clone(),
                self.data.device_id.clone(),
                self.data.device_name.clone(),
            );

            // Schedule next attempt with exponential backoff
            let next_delay = RECONNECT_BASE_DELAY_MS * 2u64.pow(reconnect.attempt - 1) / 1000;
            self.data.reconnect.as_mut().unwrap().attempt += 1;
            self.data.reconnect.as_mut().unwrap().delay_secs = next_delay;
            self.push();
        } else {
            // Peer not discovered yet — retry later
            self.data.reconnect.as_mut().unwrap().delay_secs = 5;
        }
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
                    self.data.sharing_state = SharingState::Idle;
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
                self.handle_incoming_connection(addr, write_tx, connection).await;
            }
            ConnectionEvent::HandshakeComplete { .. } => {}
            ConnectionEvent::Disconnected { reason } => {
                self.handle_disconnected(reason, connection, clipboard, injection, capture).await;
            }
            ConnectionEvent::MessageReceived(msg) => {
                self.handle_message_received(msg, connection, clipboard, injection, capture, clipboard_tx, input_tx).await;
            }
            ConnectionEvent::Error(e) => {
                error!("[continuity] connection error: {e}");
            }
        }
    }

    async fn handle_incoming_connection(
        &mut self,
        addr: std::net::SocketAddr,
        write_tx: tokio::sync::mpsc::Sender<protocol::Message>,
        connection: &mut connection::TcpConnectionProvider,
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

    async fn handle_disconnected(
        &mut self,
        reason: String,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
    ) {
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

        // Start reconnect if we had an active connection and not manually disconnected
        if let Some(conn) = was_active {
            self.start_reconnect(&conn.peer_id, &conn.peer_name);
        }
        self.push();
    }

    async fn handle_message_received(
        &mut self,
        msg: protocol::Message,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
        clipboard_tx: &Sender<clipboard::ClipboardEvent>,
        input_tx: &Sender<input::InputEvent>,
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

    async fn handle_hello(
        &mut self,
        device_id: String,
        device_name: String,
        version: u32,
        connection: &mut connection::TcpConnectionProvider,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
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
        }
    }

    async fn handle_pin_request(&mut self, pin: String) {
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

    async fn handle_pin_confirm(
        &mut self,
        pin: String,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
        clipboard_tx: &Sender<clipboard::ClipboardEvent>,
    ) {
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

    async fn handle_screen_info(
        &mut self,
        width: i32,
        height: i32,
        connection: &mut connection::TcpConnectionProvider,
        capture: &mut input::EvdevCapture,
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

    async fn handle_config_sync(
        &mut self,
        arrangement: Side,
        offset: i32,
        clipboard: bool,
        audio: bool,
        drag_drop: bool,
        version: u64,
        connection: &mut connection::TcpConnectionProvider,
    ) {
        if let Some(conn) = &self.data.active_connection {
            let peer_id = conn.peer_id.clone();
            let config = self.data.peer_configs.entry(peer_id).or_default();

            if version > config.version {
                info!("[continuity] adopting newer config from peer (v{} > v{}): {:?} offset {} clipboard={} audio={} dnd={}",
                    version, config.version, arrangement, offset, clipboard, audio, drag_drop);

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
                info!("[continuity] ignoring older/same config from peer (v{} <= v{})", version, config.version);
            }
        }
    }

    async fn handle_edge_transition(
        &mut self,
        side: Side,
        edge_pos: f64,
        connection: &mut connection::TcpConnectionProvider,
        injection: &mut input::WaylandInjection,
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

    async fn handle_transition_ack(
        &mut self,
        accepted: bool,
        connection: &mut connection::TcpConnectionProvider,
        capture: &mut input::EvdevCapture,
        input_tx: &Sender<input::InputEvent>,
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

    async fn handle_transition_cancel(&mut self) {
        info!("[continuity] forcing cursor back to local");
        self.data.sharing_state = SharingState::Idle;
        self.push();
    }

    async fn handle_switch_transition(
        &mut self,
        side: Side,
        connection: &mut connection::TcpConnectionProvider,
    ) {
        if matches!(self.data.sharing_state, SharingState::Sharing { .. }) {
            info!("[continuity] peer requesting switch via {:?}", side);
            self.data.sharing_state = SharingState::PendingSwitch;
            self.push();
            if let Some(tx) = &self.switch_tx {
                let _ = tx.try_send(ContinuityCmd::SwitchToReceiving(side));
            }
        } else {
            info!("[continuity] rejecting switch (not in Sharing, currently {:?})", self.data.sharing_state);
            connection.send_message(protocol::Message::SwitchConfirm { side, edge_pos: 0.0 });
        }
    }

    async fn handle_switch_confirm(
        &mut self,
        side: Side,
        edge_pos: f64,
        connection: &mut connection::TcpConnectionProvider,
        capture: &mut input::EvdevCapture,
        input_tx: &Sender<input::InputEvent>,
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

    async fn handle_peer_disconnect(
        &mut self,
        reason: String,
        connection: &mut connection::TcpConnectionProvider,
        clipboard: &mut clipboard::WaylandClipboard,
        injection: &mut input::WaylandInjection,
        capture: &mut input::EvdevCapture,
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
        if !matches!(self.data.sharing_state, SharingState::Sharing { .. }) {
            return;
        }
        match event {
            input::InputEvent::CursorMove { dx, dy } => {
                let (rw, rh) = self.remote_screen();
                let rw = rw as f64;
                let rh = rh as f64;

                let SharingState::Sharing { entry_side, virtual_pos: mut vpos } = self.data.sharing_state.clone() else { return };

                vpos.0 += dx;
                vpos.1 += dy;
                vpos.0 = vpos.0.clamp(-100.0, rw + 100.0);
                vpos.1 = vpos.1.clamp(-100.0, rh + 100.0);

                let should_return = match entry_side {
                    Side::Left if vpos.0 > rw => true,
                    Side::Right if vpos.0 < 0.0 => true,
                    Side::Top if vpos.1 > rh => true,
                    Side::Bottom if vpos.1 < 0.0 => true,
                    _ => false,
                };

                if should_return {
                    info!("[continuity] return transition at vpos=({:.0},{:.0})", vpos.0, vpos.1);
                    self.data.sharing_state = SharingState::Idle;
                    self.last_transition_at = Instant::now();
                    capture.stop();
                    connection.send_message(protocol::Message::TransitionCancel);
                    self.push();
                    let _ = capture.prepare();
                    return;
                }

                self.data.sharing_state = SharingState::Sharing { entry_side, virtual_pos: vpos };
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
                self.data.sharing_state = SharingState::Idle;
                capture.stop();
                connection.send_message(protocol::Message::TransitionCancel);
                self.push();
                let _ = capture.prepare();
            }
        };
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

pub use known_peers::{config_dir, hostname};

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
