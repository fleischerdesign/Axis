use std::collections::HashMap;
use std::pin::Pin;
use std::time::{Duration, Instant};

use async_channel::{Receiver, Sender, bounded};
use axis_domain::models::continuity::{
    ContinuityStatus, InputEvent, PeerArrangement, PeerConfig, Side,
};
use log::info;

use super::clipboard::{ClipboardEvent, WaylandClipboard};
use super::connection::{ConnectionEvent, TcpConnectionProvider};
use super::crypto::ContinuityCipher;
use super::discovery::{AvahiDiscovery, DiscoveryEvent};
use super::input::{EvdevCapture, InternalInputEvent, WaylandInjection};
use super::known_peers::{self, KnownPeer, KnownPeerArrangementSide, KnownPeersStore};
use super::ports::{
    ContinuityAudioPort, ContinuityCapturePort, ContinuityClipboardPort,
    ContinuityDiscoveryPort, ContinuityInjectionPort, ContinuityNetworkPort,
};

mod cmd;
mod connection;
mod discovery;
mod input;
mod reconnect;
#[cfg(test)]
mod tests;

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
    pub network: &'a mut dyn ContinuityNetworkPort,
    pub capture: &'a mut dyn ContinuityCapturePort,
    pub injection: &'a mut dyn ContinuityInjectionPort,
    pub clipboard: &'a mut dyn ContinuityClipboardPort,
    pub discovery: &'a mut dyn ContinuityDiscoveryPort,
    pub audio: &'a dyn ContinuityAudioPort,
    pub drag_drop_mgr: &'a super::drag_drop::DragDropManager,
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
    pub audio_direction: axis_domain::models::continuity::AudioStreamDirection,
    pub drag_drop: bool,
    pub version: u64,
}

pub struct ContinuityInner {
    pub(crate) status_tx: tokio::sync::watch::Sender<ContinuityStatus>,
    pub(crate) status: ContinuityStatus,
    pub(crate) cipher: std::sync::Arc<std::sync::Mutex<Option<ContinuityCipher>>>,
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
            device_id: known_peers::persistent_device_id(),
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
            cipher: std::sync::Arc::new(std::sync::Mutex::new(None)),
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
                    auto_connect: config.auto_connect,
                    clipboard: config.clipboard,
                    audio: config.audio,
                    audio_direction: config.audio_direction,
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

    pub(crate) fn create_cipher(&mut self, peer_id: &str, pin: Option<&str>) {
        let local_id = &self.status.device_id;
        let pin_str = pin.unwrap_or("trusted");
        let key = super::crypto::derive_session_key(pin_str, local_id, peer_id);
        *self.cipher.lock().unwrap() = Some(ContinuityCipher::new(&key));
    }

    pub(crate) fn encrypt_for_wire(&self, data: &[u8]) -> Vec<u8> {
        if let Some(ref mut cipher) = *self.cipher.lock().unwrap() {
            cipher.encrypt(data)
        } else {
            data.to_vec()
        }
    }

    pub(crate) fn decrypt_from_wire(&self, packet: &[u8]) -> Option<Vec<u8>> {
        if let Some(ref cipher) = *self.cipher.lock().unwrap() {
            cipher.decrypt(packet).ok()
        } else {
            Some(packet.to_vec())
        }
    }

    pub(crate) fn cipher_arc(&self) -> std::sync::Arc<std::sync::Mutex<Option<ContinuityCipher>>> {
        self.cipher.clone()
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
        let drag_drop_mgr = super::drag_drop::DragDropManager::new();
        let audio_stream_mgr = super::audio_stream::AudioStreamManager::new();
        let mut heartbeat = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        let mut reconnect_sleep: Option<Pin<Box<tokio::time::Sleep>>> = None;

        if let Ok(mut sock) = niri_ipc::socket::Socket::connect()
            && let Ok(Ok(niri_ipc::Response::Outputs(outputs))) =
                sock.send(niri_ipc::Request::Outputs)
        {
            let mut best: Option<(i32, i32)> = None;
            let mut best_area: u64 = 0;
            let mut local_geometries = Vec::new();

            for (output_name, output) in outputs {
                if let Some(logical) = &output.logical {
                    let w = logical.width as i32;
                    let h = logical.height as i32;
                    let x = logical.x;
                    let y = logical.y;

                    local_geometries.push(axis_domain::models::continuity::OutputGeometry {
                        name: output_name.clone(),
                        x,
                        y,
                        width: w,
                        height: h,
                    });

                    let covers_origin = x <= 0 && y <= 0 && x + w > 0 && y + h > 0;
                    let area = (w as u64) * (h as u64);
                    if covers_origin || area > best_area {
                        best = Some((w, h));
                        best_area = area;
                    }
                }
            }

            self.status.local_outputs = local_geometries;

            if let Some((w, h)) = best {
                info!(
                    "[continuity] detected primary output: {}x{} (logical, {} total outputs)",
                    w,
                    h,
                    self.status.local_outputs.len()
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
                        network: &mut connection,
                        capture: &mut capture,
                        injection: &mut injection,
                        clipboard: &mut clipboard,
                        discovery: &mut discovery,
                        audio: &audio_stream_mgr,
                        drag_drop_mgr: &drag_drop_mgr,
                        discovery_tx: &discovery_tx,
                        conn_tx: &conn_tx,
                        clipboard_tx: &clipboard_tx,
                        input_tx: &input_tx,
                    }).await;
                }
                Ok(cmd) = switch_rx.recv() => {
                    self.handle_cmd(cmd, &mut CmdContext {
                        network: &mut connection,
                        capture: &mut capture,
                        injection: &mut injection,
                        clipboard: &mut clipboard,
                        discovery: &mut discovery,
                        audio: &audio_stream_mgr,
                        drag_drop_mgr: &drag_drop_mgr,
                        discovery_tx: &discovery_tx,
                        conn_tx: &conn_tx,
                        clipboard_tx: &clipboard_tx,
                        input_tx: &input_tx,
                    }).await;
                }
                Ok(event) = discovery_rx.recv() => {
                    self.handle_discovery_event_with_conn(event, &mut connection, &conn_tx).await;
                }
                Ok(event) = conn_rx.recv() => {
                    let new_sleep = self.handle_connection_event(event, &mut CmdContext {
                        network: &mut connection,
                        capture: &mut capture,
                        injection: &mut injection,
                        clipboard: &mut clipboard,
                        discovery: &mut discovery,
                        audio: &audio_stream_mgr,
                        drag_drop_mgr: &drag_drop_mgr,
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
