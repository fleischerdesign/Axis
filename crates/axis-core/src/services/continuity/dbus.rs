use async_channel::Sender;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use zbus::interface;
use zbus::object_server::SignalEmitter;

use super::{ContinuityCmd, ContinuityData, PeerConfig, SharingMode};

// ── Serializable Snapshot Types ─────────────────────────────────────────
//
// These types mirror the runtime ContinuityData but are designed for
// D-Bus serialization: no Instant, no sensitive PIN data.

/// D-Bus-safe version of PeerInfo (SocketAddr serializes fine).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusPeerInfo {
    pub device_id: String,
    pub device_name: String,
    pub hostname: String,
    pub address: String,
    pub address_v6: Option<String>,
}

impl From<&super::PeerInfo> for DbusPeerInfo {
    fn from(p: &super::PeerInfo) -> Self {
        Self {
            device_id: p.device_id.clone(),
            device_name: p.device_name.clone(),
            hostname: p.hostname.clone(),
            address: p.address.to_string(),
            address_v6: p.address_v6.map(|a| a.to_string()),
        }
    }
}

/// D-Bus-safe connection info (Instant → elapsed seconds).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusConnectionInfo {
    pub peer_id: String,
    pub peer_name: String,
    pub connected_secs: u64,
}

/// D-Bus-safe PIN info (no actual PIN transmitted).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusPendingPin {
    pub peer_id: String,
    pub peer_name: String,
    pub is_incoming: bool,
}

/// Full snapshot of Continuity runtime state for D-Bus clients.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinuityStateSnapshot {
    pub enabled: bool,
    pub device_name: String,
    pub peers: Vec<DbusPeerInfo>,
    pub active_connection: Option<DbusConnectionInfo>,
    pub sharing_mode: SharingMode,
    pub pending_pin: Option<DbusPendingPin>,
    pub peer_configs: HashMap<String, PeerConfig>,
    pub screen_width: i32,
    pub screen_height: i32,
    pub remote_screen: Option<(i32, i32)>,
}

impl Default for ContinuityStateSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            device_name: String::new(),
            peers: Vec::new(),
            active_connection: None,
            sharing_mode: SharingMode::Idle,
            pending_pin: None,
            peer_configs: HashMap::new(),
            screen_width: 1920,
            screen_height: 1080,
            remote_screen: None,
        }
    }
}

/// Build a D-Bus snapshot from runtime ContinuityData.
pub fn build_snapshot(data: &ContinuityData) -> ContinuityStateSnapshot {
    ContinuityStateSnapshot {
        enabled: data.enabled,
        device_name: data.device_name.clone(),
        peers: data.peers.iter().map(DbusPeerInfo::from).collect(),
        active_connection: data.active_connection.as_ref().map(|c| DbusConnectionInfo {
            peer_id: c.peer_id.clone(),
            peer_name: c.peer_name.clone(),
            connected_secs: c.since.elapsed().as_secs(),
        }),
        sharing_mode: data.sharing_mode,
        pending_pin: data.pending_pin.as_ref().map(|p| DbusPendingPin {
            peer_id: p.peer_id.clone(),
            peer_name: p.peer_name.clone(),
            is_incoming: p.is_incoming,
        }),
        peer_configs: data.peer_configs.clone(),
        screen_width: data.screen_width,
        screen_height: data.screen_height,
        remote_screen: data.remote_screen,
    }
}

// ── D-Bus Server ────────────────────────────────────────────────────────

pub struct ContinuityDbusServer {
    cmd_tx: Sender<ContinuityCmd>,
    state: Arc<Mutex<ContinuityStateSnapshot>>,
}

impl ContinuityDbusServer {
    pub fn new(cmd_tx: Sender<ContinuityCmd>, state: Arc<Mutex<ContinuityStateSnapshot>>) -> Self {
        Self { cmd_tx, state }
    }
}

#[interface(name = "org.axis.Shell.Continuity")]
impl ContinuityDbusServer {
    /// Get the full runtime state as JSON.
    async fn get_state(&self) -> String {
        serde_json::to_string(&*self.state.lock().unwrap()).unwrap_or_default()
    }

    /// Connect to a discovered peer by device ID.
    async fn connect_to_peer(&self, peer_id: &str) -> bool {
        let _ = self.cmd_tx.try_send(ContinuityCmd::ConnectToPeer(peer_id.to_string()));
        true
    }

    /// Confirm the pending PIN challenge.
    async fn confirm_pin(&self) -> bool {
        let _ = self.cmd_tx.try_send(ContinuityCmd::ConfirmPin);
        true
    }

    /// Reject the pending PIN challenge.
    async fn reject_pin(&self) -> bool {
        let _ = self.cmd_tx.try_send(ContinuityCmd::RejectPin);
        true
    }

    /// Disconnect from the active peer.
    async fn disconnect(&self) -> bool {
        let _ = self.cmd_tx.try_send(ContinuityCmd::Disconnect);
        true
    }

    /// Unpair a known peer by device ID.
    async fn unpair(&self, peer_id: &str) -> bool {
        let _ = self.cmd_tx.try_send(ContinuityCmd::Unpair(peer_id.to_string()));
        true
    }

    /// Set peer arrangement (JSON of PeerArrangement).
    async fn set_peer_arrangement(&self, json: &str) -> bool {
        match serde_json::from_str::<super::PeerArrangement>(json) {
            Ok(arr) => {
                let _ = self.cmd_tx.try_send(ContinuityCmd::SetPeerArrangement(arr));
                true
            }
            Err(e) => {
                log::warn!("[continuity-dbus] Failed to parse arrangement: {e}");
                false
            }
        }
    }

    /// Enable or disable the continuity service.
    async fn set_enabled(&self, enabled: bool) -> bool {
        let _ = self.cmd_tx.try_send(ContinuityCmd::SetEnabled(enabled));
        true
    }

    #[zbus(property)]
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    #[zbus(signal)]
    pub async fn state_changed(
        emitter: &SignalEmitter<'_>,
        json: &str,
    ) -> zbus::Result<()>;
}
