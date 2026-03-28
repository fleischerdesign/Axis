pub mod clipboard;
pub mod connection;
pub mod discovery;
pub mod input;
pub mod protocol;

use async_channel::{bounded, Sender};
use std::time::Instant;

use super::Service;
use crate::store::ServiceStore;
use connection::{ConnectionEvent, ConnectionProvider};
use discovery::{DiscoveryEvent, DiscoveryProvider};
use log::{error, info};

// ── Constants ──────────────────────────────────────────────────────────

pub const CONTINUITY_PORT: u16 = 7391;
const HEARTBEAT_INTERVAL_SECS: u64 = 5;
const CONNECTION_TIMEOUT_SECS: u64 = 15;
pub const PIN_LENGTH: usize = 6;

// ── Data Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharingMode {
    Idle,
    Sharing,
    Receiving,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeerInfo {
    pub device_id: String,
    pub device_name: String,
    pub address: std::net::SocketAddr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveConnectionInfo {
    pub peer_id: String,
    pub peer_name: String,
    pub since: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContinuityData {
    pub device_id: String,
    pub device_name: String,
    pub enabled: bool,
    pub peers: Vec<PeerInfo>,
    pub active_connection: Option<ActiveConnectionInfo>,
    pub sharing_mode: SharingMode,
    pub pending_pin: Option<String>,
}

impl Default for ContinuityData {
    fn default() -> Self {
        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            device_name: hostname(),
            enabled: false,
            peers: Vec::new(),
            active_connection: None,
            sharing_mode: SharingMode::Idle,
            pending_pin: None,
        }
    }
}

// ── Commands ───────────────────────────────────────────────────────────

pub enum ContinuityCmd {
    ToggleEnabled,
    ConnectToPeer(String),
    ConfirmPin(String),
    RejectPin,
    Disconnect,
    ForceLocal,
}

// ── Service ────────────────────────────────────────────────────────────

pub struct ContinuityService;

impl Service for ContinuityService {
    type Data = ContinuityData;
    type Cmd = ContinuityCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(32);
        let (cmd_tx, cmd_rx) = bounded(32);

        tokio::spawn(async move {
            let mut service = ContinuityInner::new(data_tx);
            service.run(cmd_rx).await;
        });

        let store = ServiceStore::new(data_rx, ContinuityData::default());
        (store, cmd_tx)
    }
}

// ── Internal State ─────────────────────────────────────────────────────

struct ContinuityInner {
    data_tx: Sender<ContinuityData>,
    data: ContinuityData,
}

impl ContinuityInner {
    fn new(data_tx: Sender<ContinuityData>) -> Self {
        Self {
            data_tx,
            data: ContinuityData::default(),
        }
    }

    fn push(&self) {
        let _ = self.data_tx.try_send(self.data.clone());
    }

    async fn run(&mut self, cmd_rx: async_channel::Receiver<ContinuityCmd>) {
        use tokio::select;

        let (discovery_tx, discovery_rx) = bounded::<DiscoveryEvent>(32);
        let (conn_tx, conn_rx) = bounded::<ConnectionEvent>(64);

        let mut discovery = discovery::AvahiDiscovery::new();
        let mut connection = connection::TcpConnectionProvider::new();

        info!("[continuity] service started, device: {}", self.data.device_name);

        loop {
            select! {
                Ok(cmd) = cmd_rx.recv() => {
                    self.handle_cmd(cmd, &mut discovery, &mut connection, &discovery_tx, &conn_tx).await;
                }
                Ok(event) = discovery_rx.recv() => {
                    self.handle_discovery_event(event).await;
                }
                Ok(event) = conn_rx.recv() => {
                    self.handle_connection_event(event, &mut connection).await;
                }
            }
        }
    }

    async fn handle_cmd(
        &mut self,
        cmd: ContinuityCmd,
        discovery: &mut discovery::AvahiDiscovery,
        connection: &mut connection::TcpConnectionProvider,
        discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
    ) {
        match cmd {
            ContinuityCmd::ToggleEnabled => {
                self.data.enabled = !self.data.enabled;
                if self.data.enabled {
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
                    self.data.peers.clear();
                    self.data.active_connection = None;
                    self.data.sharing_mode = SharingMode::Idle;
                    self.data.pending_pin = None;
                }
                self.push();
            }
            ContinuityCmd::ConnectToPeer(peer_id) => {
                if let Some(peer) = self.data.peers.iter().find(|p| p.device_id == peer_id) {
                    let addr = peer.address;
                    let name = peer.device_name.clone();
                    info!("[continuity] connecting to {name} at {addr}");
                    if let Err(e) = connection.connect(addr, conn_tx.clone()) {
                        error!("[continuity] connect failed: {e}");
                    }
                }
            }
            ContinuityCmd::ConfirmPin(pin) => {
                info!("[continuity] PIN confirmed: {pin}");
                self.data.pending_pin = None;
                // TODO: send PinConfirm message via connection
                self.push();
            }
            ContinuityCmd::RejectPin => {
                info!("[continuity] PIN rejected");
                self.data.pending_pin = None;
                // TODO: send Disconnect message
                self.push();
            }
            ContinuityCmd::Disconnect => {
                info!("[continuity] disconnecting");
                connection.disconnect_active();
                self.data.active_connection = None;
                self.data.sharing_mode = SharingMode::Idle;
                self.push();
            }
            ContinuityCmd::ForceLocal => {
                if self.data.sharing_mode == SharingMode::Sharing {
                    info!("[continuity] forcing cursor back to local");
                    self.data.sharing_mode = SharingMode::Idle;
                    // TODO: send TransitionCancel message
                    self.push();
                }
            }
        }
    }

    async fn handle_discovery_event(&mut self, event: DiscoveryEvent) {
        match event {
            DiscoveryEvent::PeerFound(peer) => {
                // Skip ourselves (match by hostname) and duplicates
                if peer.device_id == self.data.device_name {
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
                }
                self.push();
            }
        }
    }

    async fn handle_connection_event(
        &mut self,
        event: ConnectionEvent,
        connection: &mut connection::TcpConnectionProvider,
    ) {
        match event {
            ConnectionEvent::IncomingConnection { peer_name, .. } => {
                info!("[continuity] incoming connection from {peer_name}");
                // TODO: start handshake, generate PIN
                self.push();
            }
            ConnectionEvent::HandshakeComplete { peer_id, peer_name } => {
                info!("[continuity] handshake complete with {peer_name}");
                self.data.active_connection = Some(ActiveConnectionInfo {
                    peer_id,
                    peer_name,
                    since: Instant::now(),
                });
                self.data.sharing_mode = SharingMode::Idle;
                self.push();
            }
            ConnectionEvent::Disconnected { reason } => {
                info!("[continuity] disconnected: {reason}");
                self.data.active_connection = None;
                self.data.sharing_mode = SharingMode::Idle;
                self.push();
            }
            ConnectionEvent::MessageReceived(msg) => {
                // TODO: handle protocol messages
                info!("[continuity] received: {:?}", msg);
            }
            ConnectionEvent::Error(e) => {
                error!("[continuity] connection error: {e}");
            }
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "axis-device".into())
}
