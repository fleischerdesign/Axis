pub mod clipboard;
pub mod cmd_handlers;
pub mod connection;
pub mod connection_handlers;
pub mod dbus;
pub mod discovery;
pub mod event_handlers;
pub mod input;
pub mod known_peers;
pub mod protocol;
pub mod reconnect;
pub mod sharing;

use async_channel::{bounded, Sender};
use std::time::{Instant, Duration};

use super::{Service, ServiceConfig};
use crate::store::ServiceStore;
use clipboard::ClipboardSync;
use input::{InputCapture, InputInjection};
use connection::{ConnectionEvent, ConnectionProvider};
pub use protocol::Side;
use discovery::{DiscoveryEvent, DiscoveryProvider};
use log::{error, info};

// ── Constants ──────────────────────────────────────────────────────────

pub const CONTINUITY_PORT: u16 = 7391;
const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const CONNECTION_TIMEOUT_SECS: u64 = 15;
pub const PIN_LENGTH: usize = 6;
const RECONNECT_MAX_ATTEMPTS: u32 = 5;
const RECONNECT_BASE_DELAY_MS: u64 = 1000;
const PIN_EXPIRY_SECS: u64 = 30;
const VIRTUAL_POS_BUFFER: f64 = 40.0;

// ── Data Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SharingState {
    Idle,
    Pending {
        entry_side: Side,
        edge_pos: f64,
    },
    Sharing {
        entry_side: Side,
        virtual_pos: (f64, f64),
    },
    Receiving,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PeerArrangement {
    pub side: Side,
    pub offset: i32,
}

impl PeerArrangement {
    pub fn overlap_on_local(&self, local_len: i32, remote_len: i32) -> Option<(i32, i32)> {
        let start = self.offset.max(0);
        let end = (self.offset + remote_len).min(local_len);
        if start < end { Some((start, end)) } else { None }
    }

    pub fn overlap_on_remote(&self, local_len: i32, remote_len: i32) -> Option<(i32, i32)> {
        let start = (-self.offset).max(0);
        let end = (local_len - self.offset).min(remote_len);
        if start < end { Some((start, end)) } else { None }
    }

    pub fn local_to_remote_edge(&self, local_pos: f64) -> f64 {
        local_pos - self.offset as f64
    }

    pub fn remote_to_local_edge(&self, remote_pos: f64) -> f64 {
        remote_pos + self.offset as f64
    }

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

    fn push(&self) {
        let _ = self.data_tx.try_send(self.data.clone());
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

        if let Ok(mut sock) = niri_ipc::socket::Socket::connect() {
            if let Ok(Ok(niri_ipc::Response::Outputs(outputs))) = sock.send(niri_ipc::Request::Outputs) {
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
