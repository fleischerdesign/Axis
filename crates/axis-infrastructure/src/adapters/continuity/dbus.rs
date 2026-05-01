use async_channel::Sender;
use axis_domain::models::continuity::{
    ContinuityStatus, PeerArrangement, PeerConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::watch;

use super::inner::ContinuityCmd;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusPeerInfo {
    pub device_id: String,
    pub device_name: String,
    pub hostname: String,
    pub address: String,
    pub address_v6: Option<String>,
}

impl From<&axis_domain::models::continuity::PeerInfo> for DbusPeerInfo {
    fn from(p: &axis_domain::models::continuity::PeerInfo) -> Self {
        Self {
            device_id: p.device_id.clone(),
            device_name: p.device_name.clone(),
            hostname: p.hostname.clone(),
            address: p.address.to_string(),
            address_v6: p.address_v6.map(|a| a.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusConnectionInfo {
    pub peer_id: String,
    pub peer_name: String,
    pub connected_secs: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusPendingPin {
    pub peer_id: String,
    pub peer_name: String,
    pub is_incoming: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusReconnectState {
    pub peer_name: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_secs: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SharingStateLabel {
    Idle,
    Pending,
    Sharing,
    Receiving,
    PendingSwitch,
}

impl From<&axis_domain::models::continuity::SharingState> for SharingStateLabel {
    fn from(state: &axis_domain::models::continuity::SharingState) -> Self {
        match state {
            axis_domain::models::continuity::SharingState::Idle => Self::Idle,
            axis_domain::models::continuity::SharingState::Pending { .. } => Self::Pending,
            axis_domain::models::continuity::SharingState::Sharing { .. } => Self::Sharing,
            axis_domain::models::continuity::SharingState::Receiving => Self::Receiving,
            axis_domain::models::continuity::SharingState::PendingSwitch => Self::PendingSwitch,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContinuityStateSnapshot {
    pub enabled: bool,
    pub device_name: String,
    pub peers: Vec<DbusPeerInfo>,
    pub active_connection: Option<DbusConnectionInfo>,
    pub sharing_state: SharingStateLabel,
    pub pending_pin: Option<DbusPendingPin>,
    pub peer_configs: HashMap<String, PeerConfig>,
    pub screen_width: i32,
    pub screen_height: i32,
    pub remote_screen: Option<(i32, i32)>,
    pub reconnect: Option<DbusReconnectState>,
}

impl Default for ContinuityStateSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            device_name: String::new(),
            peers: Vec::new(),
            active_connection: None,
            sharing_state: SharingStateLabel::Idle,
            pending_pin: None,
            peer_configs: HashMap::new(),
            screen_width: 1920,
            screen_height: 1080,
            remote_screen: None,
            reconnect: None,
        }
    }
}

pub fn build_snapshot(status: &ContinuityStatus) -> ContinuityStateSnapshot {
    ContinuityStateSnapshot {
        enabled: status.enabled,
        device_name: status.device_name.clone(),
        peers: status.peers.iter().map(DbusPeerInfo::from).collect(),
        active_connection: status.active_connection.as_ref().map(|c| DbusConnectionInfo {
            peer_id: c.peer_id.clone(),
            peer_name: c.peer_name.clone(),
            connected_secs: c.connected_secs,
        }),
        sharing_state: (&status.sharing_state).into(),
        pending_pin: status.pending_pin.as_ref().map(|p| DbusPendingPin {
            peer_id: p.peer_id.clone(),
            peer_name: p.peer_name.clone(),
            is_incoming: p.is_incoming,
        }),
        peer_configs: status.peer_configs.clone(),
        screen_width: status.screen_width,
        screen_height: status.screen_height,
        remote_screen: status.remote_screen,
        reconnect: status.reconnect.as_ref().map(|r| DbusReconnectState {
            peer_name: r.peer_name.clone(),
            attempt: r.attempt,
            max_attempts: r.max_attempts,
            delay_secs: r.delay_secs,
        }),
    }
}

pub struct ContinuityDbusServer {
    pub cmd_tx: Sender<ContinuityCmd>,
    state_rx: watch::Receiver<ContinuityStatus>,
}

impl ContinuityDbusServer {
    pub fn new(
        cmd_tx: Sender<ContinuityCmd>,
        state_rx: watch::Receiver<ContinuityStatus>,
    ) -> Self {
        Self { cmd_tx, state_rx }
    }
}

#[zbus::interface(name = "org.axis.Shell.Continuity")]
impl ContinuityDbusServer {
    async fn get_state(&self) -> String {
        serde_json::to_string(&*self.state_rx.borrow()).unwrap_or_default()
    }

    async fn connect_to_peer(&self, peer_id: &str) -> bool {
        self.cmd_tx
            .try_send(ContinuityCmd::ConnectToPeer(peer_id.to_string()))
            .is_ok()
    }

    async fn confirm_pin(&self) -> bool {
        self.cmd_tx.try_send(ContinuityCmd::ConfirmPin).is_ok()
    }

    async fn reject_pin(&self) -> bool {
        self.cmd_tx.try_send(ContinuityCmd::RejectPin).is_ok()
    }

    async fn disconnect(&self) -> bool {
        self.cmd_tx.try_send(ContinuityCmd::Disconnect).is_ok()
    }

    async fn cancel_reconnect(&self) -> bool {
        self.cmd_tx.try_send(ContinuityCmd::CancelReconnect).is_ok()
    }

    async fn unpair(&self, peer_id: &str) -> bool {
        self.cmd_tx
            .try_send(ContinuityCmd::Unpair(peer_id.to_string()))
            .is_ok()
    }

    async fn set_peer_arrangement(&self, json: &str) -> bool {
        match serde_json::from_str::<PeerArrangement>(json) {
            Ok(arr) => self
                .cmd_tx
                .try_send(ContinuityCmd::SetPeerArrangement(arr))
                .is_ok(),
            Err(e) => {
                log::warn!("[continuity-dbus] Failed to parse arrangement: {e}");
                false
            }
        }
    }

    async fn set_enabled(&self, enabled: bool) -> bool {
        self.cmd_tx
            .try_send(ContinuityCmd::SetEnabled(enabled))
            .is_ok()
    }

    async fn update_peer_configs(&self, json: &str) -> bool {
        match serde_json::from_str::<HashMap<String, PeerConfig>>(json) {
            Ok(configs) => self
                .cmd_tx
                .try_send(ContinuityCmd::UpdatePeerConfigs(configs))
                .is_ok(),
            Err(e) => {
                log::warn!("[continuity-dbus] Failed to parse peer configs: {e}");
                false
            }
        }
    }

    #[zbus(property)]
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    #[zbus(signal)]
    pub async fn state_changed(emitter: &zbus::object_server::SignalEmitter<'_>, json: &str) -> zbus::Result<()>;
}
