pub mod clipboard;
pub mod connection;
pub mod discovery;
pub mod input;
pub mod protocol;

use async_channel::{bounded, Sender};
use std::time::Instant;

use super::Service;
use crate::store::ServiceStore;
use clipboard::ClipboardSync;
use connection::{ConnectionEvent, ConnectionProvider};
use discovery::{DiscoveryEvent, DiscoveryProvider};
use log::{error, info, warn};

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

#[derive(Debug, Clone, PartialEq)]
pub struct ContinuityData {
    pub device_id: String,
    pub device_name: String,
    pub enabled: bool,
    pub peers: Vec<PeerInfo>,
    pub active_connection: Option<ActiveConnectionInfo>,
    pub sharing_mode: SharingMode,
    pub pending_pin: Option<PendingPin>,
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
    ConfirmPin,
    RejectPin,
    Disconnect,
    ForceLocal,
    StartDiscovery,
    StopDiscovery,
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
    last_message_at: Option<Instant>,
    is_initiating: bool,
    pending_peer: Option<(String, String)>,
}

impl ContinuityInner {
    fn new(data_tx: Sender<ContinuityData>) -> Self {
        Self {
            data_tx,
            data: ContinuityData::default(),
            last_message_at: None,
            is_initiating: false,
            pending_peer: None,
        }
    }

    fn push(&self) {
        let _ = self.data_tx.try_send(self.data.clone());
    }

    async fn run(&mut self, cmd_rx: async_channel::Receiver<ContinuityCmd>) {
        use tokio::select;
        use tokio::time::{interval, Duration};

        let (discovery_tx, discovery_rx) = bounded::<DiscoveryEvent>(32);
        let (conn_tx, conn_rx) = bounded::<ConnectionEvent>(64);
        let (clipboard_tx, clipboard_rx) = bounded::<clipboard::ClipboardEvent>(32);

        let mut discovery = discovery::AvahiDiscovery::new();
        let mut connection = connection::TcpConnectionProvider::new();
        let mut clipboard = clipboard::WaylandClipboard::new();
        let mut heartbeat = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));

        info!("[continuity] service started, device: {}", self.data.device_name);

        loop {
            let is_connected = self.data.active_connection.is_some();

            select! {
                Ok(cmd) = cmd_rx.recv() => {
                    self.handle_cmd(cmd, &mut discovery, &mut connection, &mut clipboard, &discovery_tx, &conn_tx, &clipboard_tx).await;
                }
                Ok(event) = discovery_rx.recv() => {
                    self.handle_discovery_event(event).await;
                }
                Ok(event) = conn_rx.recv() => {
                    self.handle_connection_event(event, &mut connection, &mut clipboard, &clipboard_tx).await;
                }
                Ok(event) = clipboard_rx.recv() => {
                    self.handle_clipboard_event(event, &connection).await;
                }
                _ = heartbeat.tick(), if is_connected => {
                    self.handle_heartbeat(&mut connection);
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
        discovery_tx: &Sender<DiscoveryEvent>,
        conn_tx: &Sender<ConnectionEvent>,
        clipboard_tx: &Sender<clipboard::ClipboardEvent>,
    ) {
        match cmd {
            ContinuityCmd::ToggleEnabled => {
                self.data.enabled = !self.data.enabled;
                if self.data.enabled {
                    info!("[continuity] enabled");
                    if let Err(e) = discovery.register(&self.data.device_name, CONTINUITY_PORT) {
                        error!("[continuity] discovery register failed: {e}");
                    }
                    if let Err(e) = connection.listen(CONTINUITY_PORT, conn_tx.clone()) {
                        error!("[continuity] listen failed: {e}");
                    }
                } else {
                    info!("[continuity] disabled");
                    discovery.stop();
                    connection.stop();
                    clipboard.stop_monitoring();
                    self.data.peers.clear();
                    self.data.active_connection = None;
                    self.data.sharing_mode = SharingMode::Idle;
                    self.data.pending_pin = None;
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

                        // Start clipboard sync
                        if let Err(e) = clipboard.start_monitoring(clipboard_tx.clone()) {
                            error!("[continuity] failed to start clipboard monitoring: {e}");
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
                self.data.active_connection = None;
                self.data.sharing_mode = SharingMode::Idle;
                self.push();
            }
            ContinuityCmd::Disconnect => {
                info!("[continuity] disconnecting");
                connection.disconnect_active();
                clipboard.stop_monitoring();
                self.data.active_connection = None;
                self.data.sharing_mode = SharingMode::Idle;
                self.last_message_at = None;
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

    fn handle_heartbeat(&mut self, connection: &mut connection::TcpConnectionProvider) {
        if let Some(last) = self.last_message_at {
            if last.elapsed().as_secs() > CONNECTION_TIMEOUT_SECS {
                warn!("[continuity] peer timed out (no message for {CONNECTION_TIMEOUT_SECS}s)");
                connection.disconnect_active();
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
        clipboard_tx: &Sender<clipboard::ClipboardEvent>,
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
            ConnectionEvent::HandshakeComplete { peer_id, peer_name } => {
                // Not used much with the new flow, handled inside MessageReceived
            }
            ConnectionEvent::Disconnected { reason } => {
                info!("[continuity] disconnected: {reason}");
                self.data.active_connection = None;
                self.data.sharing_mode = SharingMode::Idle;
                self.data.pending_pin = None;
                self.pending_peer = None;
                self.last_message_at = None;
                clipboard.stop_monitoring();
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

                                // Start clipboard sync
                                if let Err(e) = clipboard.start_monitoring(clipboard_tx.clone()) {
                                    error!("[continuity] failed to start clipboard monitoring: {e}");
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
                        self.push();
                    }
                    other => {
                        info!("[continuity] received: {:?}", other);
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
