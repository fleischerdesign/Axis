use std::collections::HashMap;
use std::pin::Pin;
use std::time::{Duration, Instant};
use uuid::Uuid;

use async_channel::{Receiver, Sender, bounded};
use axis_domain::models::continuity::{
    ContinuityStatus, InputEvent, PeerArrangement, PeerConfig, Side,
};
use log::info;

use super::clipboard::{ClipboardEvent, WaylandClipboard};
use super::connection::{ConnectionEvent, TcpConnectionProvider};
use super::discovery::{AvahiDiscovery, DiscoveryEvent};
use super::input::{EvdevCapture, InternalInputEvent, WaylandInjection};
use super::known_peers::{self, KnownPeer, KnownPeerArrangementSide, KnownPeersStore};

mod cmd;
mod connection;
mod discovery;
mod input;
mod reconnect;

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

pub(crate) struct CmdContext<'a> {
    pub discovery: &'a mut AvahiDiscovery,
    pub connection: &'a mut TcpConnectionProvider,
    pub clipboard: &'a mut WaylandClipboard,
    pub injection: &'a mut WaylandInjection,
    pub capture: &'a mut EvdevCapture,
    pub discovery_tx: &'a Sender<DiscoveryEvent>,
    pub conn_tx: &'a Sender<ConnectionEvent>,
    pub clipboard_tx: &'a Sender<ClipboardEvent>,
    pub input_tx: &'a Sender<InternalInputEvent>,
}

pub(crate) struct ConfigSyncArgs {
    pub arrangement: Side,
    pub offset: i32,
    pub clipboard: bool,
    pub audio: bool,
    pub drag_drop: bool,
    pub version: u64,
}

pub struct ContinuityInner {
    pub(crate) status_tx: tokio::sync::watch::Sender<ContinuityStatus>,
    pub(crate) status: ContinuityStatus,
    pub(crate) connected_at: Option<Instant>,
    pub(crate) pin_created_at: Option<Instant>,
    pub(crate) last_message_at: Option<Instant>,
    pub(crate) is_initiating: bool,
    pub(crate) pending_peer: Option<(String, String)>,
    pub(crate) last_transition_at: Instant,
    pub(crate) switch_tx: Option<Sender<ContinuityCmd>>,
    pub(crate) known_peers: KnownPeersStore,
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
            status
                .peer_configs
                .insert(id.clone(), known_peer.to_peer_config());
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

    pub(crate) fn push(&self) {
        let mut status = self.status.clone();
        if let (Some(conn), Some(started)) = (&mut status.active_connection, &self.connected_at) {
            conn.connected_secs = started.elapsed().as_secs();
        }
        let _ = self.status_tx.send(status);
    }

    pub(crate) fn persist_known_peers(&self) {
        let peers: HashMap<_, _> = self
            .status
            .peer_configs
            .iter()
            .filter(|(_, c)| c.trusted)
            .map(|(id, config)| {
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
                    .unwrap_or_else(|| (String::new(), String::new(), String::new(), None));

                let (arrangement_side, arrangement_x, arrangement_y) = match config.arrangement.side
                {
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
                (id.clone(), known_peer)
            })
            .collect();

        known_peers::save_known_peers(&KnownPeersStore { peers });
    }

    pub(crate) fn remote_screen(&self) -> (i32, i32) {
        self.status
            .remote_screen
            .unwrap_or((self.status.screen_width, self.status.screen_height))
    }

    pub(crate) fn init_virtual_pos(
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
        use tokio::time::{Duration, interval};

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

        if let Ok(mut sock) = niri_ipc::socket::Socket::connect()
            && let Ok(Ok(niri_ipc::Response::Outputs(outputs))) =
                sock.send(niri_ipc::Request::Outputs)
        {
            let mut best: Option<(i32, i32)> = None;
            let mut best_area: u64 = 0;
            for output in outputs.values() {
                if let Some(logical) = &output.logical {
                    let w = logical.width as i32;
                    let h = logical.height as i32;
                    let covers_origin =
                        logical.x <= 0 && logical.y <= 0 && logical.x + w > 0 && logical.y + h > 0;
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

        info!(
            "[continuity] service started, device: {}",
            self.status.device_name
        );

        loop {
            let is_connected = self.status.active_connection.is_some();

            select! {
                Ok(cmd) = cmd_rx.recv() => {
                    self.handle_cmd(cmd, &mut CmdContext {
                        discovery: &mut discovery,
                        connection: &mut connection,
                        clipboard: &mut clipboard,
                        injection: &mut injection,
                        capture: &mut capture,
                        discovery_tx: &discovery_tx,
                        conn_tx: &conn_tx,
                        clipboard_tx: &clipboard_tx,
                        input_tx: &input_tx,
                    }).await;
                }
                Ok(cmd) = switch_rx.recv() => {
                    self.handle_cmd(cmd, &mut CmdContext {
                        discovery: &mut discovery,
                        connection: &mut connection,
                        clipboard: &mut clipboard,
                        injection: &mut injection,
                        capture: &mut capture,
                        discovery_tx: &discovery_tx,
                        conn_tx: &conn_tx,
                        clipboard_tx: &clipboard_tx,
                        input_tx: &input_tx,
                    }).await;
                }
                Ok(event) = discovery_rx.recv() => {
                    self.handle_discovery_event(event).await;
                }
                Ok(event) = conn_rx.recv() => {
                    let new_sleep = self.handle_connection_event(event, &mut CmdContext {
                        discovery: &mut discovery,
                        connection: &mut connection,
                        clipboard: &mut clipboard,
                        injection: &mut injection,
                        capture: &mut capture,
                        discovery_tx: &discovery_tx,
                        conn_tx: &conn_tx,
                        clipboard_tx: &clipboard_tx,
                        input_tx: &input_tx,
                    }).await;
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
