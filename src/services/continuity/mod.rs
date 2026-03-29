pub mod clipboard;
pub mod connection;
pub mod discovery;
pub mod input;
pub mod protocol;

use async_channel::{bounded, Sender};
use std::time::{Instant, Duration};

use super::Service;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharingMode {
    Idle,
    Pending,
    Sharing,
    Receiving,
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

#[derive(Debug, Clone, PartialEq)]
pub struct ContinuityData {
    pub device_id: String,
    pub device_name: String,
    pub enabled: bool,
    pub peers: Vec<PeerInfo>,
    pub active_connection: Option<ActiveConnectionInfo>,
    pub sharing_mode: SharingMode,
    pub pending_pin: Option<PendingPin>,
    pub preferred_edge: Side,
    pub screen_width: i32,
    pub screen_height: i32,
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
            preferred_edge: Side::Right,
            screen_width: 1920,
            screen_height: 1080,
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
    StartSharing(Side),
    StopSharing,
    SendInput(protocol::Message),
    SetPreferredEdge(Side),
    SetScreenSize(i32, i32),
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
    virtual_pos: (f64, f64),
    entry_side: Option<Side>,
    pending_transition_side: Option<Side>,
    last_transition_at: Instant,
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
            last_transition_at: Instant::now() - Duration::from_secs(10),
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
        let (input_tx, input_rx) = bounded::<input::InputEvent>(128);

        let mut discovery = discovery::AvahiDiscovery::new();
        let mut connection = connection::TcpConnectionProvider::new();
        let mut clipboard = clipboard::WaylandClipboard::new();
        let mut injection = input::WaylandInjection::new();
        let mut capture = input::EvdevCapture::new();
        let mut heartbeat = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));

        // Get initial screen dimensions from Niri (physical pixel mode)
        if let Ok(mut sock) = niri_ipc::socket::Socket::connect() {
            if let Ok(Ok(niri_ipc::Response::Outputs(outputs))) = sock.send(niri_ipc::Request::Outputs) {
                // Pick the first enabled output with a current mode
                if let Some(output) = outputs.values().find(|o| o.logical.is_some()) {
                    if let Some(mode_idx) = output.current_mode {
                        if let Some(mode) = output.modes.get(mode_idx) {
                            let w = mode.width as i32;
                            let h = mode.height as i32;
                            info!("[continuity] detected physical resolution: {}x{}", w, h);
                            self.data.screen_width = w;
                            self.data.screen_height = h;
                        }
                    }
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
                    injection.stop();
                    capture.stop();
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
                if self.data.sharing_mode == SharingMode::Sharing || self.data.sharing_mode == SharingMode::Pending {
                    info!("[continuity] forcing cursor back to local");
                    self.data.sharing_mode = SharingMode::Idle;
                    self.pending_transition_side = None;
                    capture.stop();
                    connection.send_message(protocol::Message::TransitionCancel);
                    self.push();
                }
            }
            ContinuityCmd::StartSharing(side) => {
                if self.data.active_connection.is_some() && self.data.sharing_mode == SharingMode::Idle {
                    // 2 second cooldown to prevent jitter
                    if self.last_transition_at.elapsed() < Duration::from_secs(2) {
                        return;
                    }

                    info!("[continuity] initiating sharing via {:?}, waiting for ack", side);
                    self.data.sharing_mode = SharingMode::Pending;
                    self.pending_transition_side = Some(side);
                    self.last_transition_at = Instant::now();

                    connection.send_message(protocol::Message::EdgeTransition { side });
                    self.push();
                }
            }
            ContinuityCmd::StopSharing => {
                if self.data.sharing_mode == SharingMode::Sharing || self.data.sharing_mode == SharingMode::Pending {
                    info!("[continuity] stopping sharing");
                    self.data.sharing_mode = SharingMode::Idle;
                    self.pending_transition_side = None;
                    capture.stop();
                    connection.send_message(protocol::Message::TransitionCancel);
                    self.push();
                }
            }
            ContinuityCmd::SendInput(msg) => {
                if self.data.sharing_mode == SharingMode::Sharing {
                    connection.send_message(msg);
                }
            }
            ContinuityCmd::SetPreferredEdge(side) => {
                self.data.preferred_edge = side;
                self.push();
            }
            ContinuityCmd::SetScreenSize(w, h) => {
                info!("[continuity] screen size set to {}x{}", w, h);
                self.data.screen_width = w;
                self.data.screen_height = h;
                self.push();
            }
        }
    }

    fn handle_heartbeat(&mut self, connection: &mut connection::TcpConnectionProvider, capture: &mut input::EvdevCapture) {
        if let Some(last) = self.last_message_at {
            // Use much shorter timeout if we are currently being controlled (Receiving),
            // so the user isn't "trapped" if the other side crashes.
            let timeout = if self.data.sharing_mode == SharingMode::Receiving {
                Duration::from_secs(5)
            } else if self.data.sharing_mode == SharingMode::Pending {
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
                self.pending_peer = None;
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
                    protocol::Message::EdgeTransition { side } => {
                        if self.data.sharing_mode == SharingMode::Idle {
                            info!("[continuity] accepting sharing from peer via {:?}", side);
                            self.data.sharing_mode = SharingMode::Receiving;
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
                                info!("[continuity] transition accepted, starting sharing via {:?}", side);
                                self.data.sharing_mode = SharingMode::Sharing;
                                self.entry_side = Some(side);
                                self.pending_transition_side = None;

                                // Start cursor at the edge we just crossed, with a buffer
                                let buffer = 500.0;
                                self.virtual_pos = match side {
                                    Side::Left => (self.data.screen_width as f64 - buffer, self.data.screen_height as f64 / 2.0),
                                    Side::Right => (buffer, self.data.screen_height as f64 / 2.0),
                                    Side::Top => (self.data.screen_width as f64 / 2.0, self.data.screen_height as f64 - buffer),
                                    Side::Bottom => (self.data.screen_width as f64 / 2.0, buffer),
                                };

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
                        info!("[continuity] peer cancelled sharing");
                        self.data.sharing_mode = SharingMode::Idle;
                        self.pending_transition_side = None;
                        self.push();
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
                    // Update virtual position
                    self.virtual_pos.0 += dx;
                    self.virtual_pos.1 += dy;

                    // Clamping
                    self.virtual_pos.0 = self.virtual_pos.0.clamp(-100.0, self.data.screen_width as f64 + 100.0);
                    self.virtual_pos.1 = self.virtual_pos.1.clamp(-100.0, self.data.screen_height as f64 + 100.0);

                    // Check for return transition
                    if let Some(entry) = self.entry_side {
                        let should_return = match entry {
                            Side::Left if self.virtual_pos.0 > self.data.screen_width as f64 => true,
                            Side::Right if self.virtual_pos.0 < 0.0 => true,
                            Side::Top if self.virtual_pos.1 > self.data.screen_height as f64 => true,
                            Side::Bottom if self.virtual_pos.1 < 0.0 => true,
                            _ => false,
                        };

                        if should_return {
                            info!("[continuity] return transition triggered via movement");
                            self.data.sharing_mode = SharingMode::Idle;
                            self.entry_side = None;
                            // Reset cooldown timer to prevent immediate re-transition
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
