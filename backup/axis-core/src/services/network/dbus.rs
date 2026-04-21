use async_channel::Sender;
use serde::{Deserialize, Serialize};
use zbus::interface;
use zbus::object_server::SignalEmitter;

use super::{NetworkData, AccessPointData, NetworkCmd};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusAccessPoint {
    pub path: String,
    pub ssid: String,
    pub strength: u8,
    pub is_active: bool,
    pub needs_auth: bool,
}

impl From<&AccessPointData> for DbusAccessPoint {
    fn from(ap: &AccessPointData) -> Self {
        Self {
            path: ap.path.clone(),
            ssid: ap.ssid.clone(),
            strength: ap.strength,
            is_active: ap.is_active,
            needs_auth: ap.needs_auth,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkStateSnapshot {
    pub is_wifi_enabled: bool,
    pub is_wifi_connected: bool,
    pub is_ethernet_connected: bool,
    pub active_ssid: Option<String>,
    pub active_strength: u8,
    pub access_points: Vec<DbusAccessPoint>,
    pub is_scanning: bool,
}

impl Default for NetworkStateSnapshot {
    fn default() -> Self {
        Self {
            is_wifi_enabled: false,
            is_wifi_connected: false,
            is_ethernet_connected: false,
            active_ssid: None,
            active_strength: 0,
            access_points: Vec::new(),
            is_scanning: false,
        }
    }
}

pub fn build_snapshot(data: &NetworkData) -> NetworkStateSnapshot {
    let active_ssid = data.access_points.iter()
        .find(|ap| ap.is_active)
        .map(|ap| ap.ssid.clone());
    
    NetworkStateSnapshot {
        is_wifi_enabled: data.is_wifi_enabled,
        is_wifi_connected: data.is_wifi_connected,
        is_ethernet_connected: data.is_ethernet_connected,
        active_ssid,
        active_strength: data.active_strength,
        access_points: data.access_points.iter().map(DbusAccessPoint::from).collect(),
        is_scanning: data.is_scanning,
    }
}

pub struct NetworkDbusServer {
    cmd_tx: Sender<NetworkCmd>,
    state_rx: tokio::sync::watch::Receiver<NetworkStateSnapshot>,
}

impl NetworkDbusServer {
    pub fn new(cmd_tx: Sender<NetworkCmd>, state_rx: tokio::sync::watch::Receiver<NetworkStateSnapshot>) -> Self {
        Self { cmd_tx, state_rx }
    }
}

#[interface(name = "org.axis.Shell.Network")]
impl NetworkDbusServer {
    async fn get_state(&self) -> String {
        serde_json::to_string(&*self.state_rx.borrow()).unwrap_or_default()
    }

    async fn set_wifi_enabled(&self, enabled: bool) -> bool {
        self.cmd_tx.try_send(NetworkCmd::ToggleWifi(enabled)).is_ok()
    }

    async fn scan_wifi(&self) -> bool {
        self.cmd_tx.try_send(NetworkCmd::ScanWifi).is_ok()
    }

    async fn connect_ap(&self, path: &str) -> bool {
        self.cmd_tx.try_send(NetworkCmd::ConnectToAp(path.to_string())).is_ok()
    }

    async fn connect_ap_with_password(&self, path: &str, ssid: &str, password: &str) -> bool {
        self.cmd_tx.try_send(NetworkCmd::ConnectToApWithPassword(
            path.to_string(),
            ssid.to_string(),
            password.to_string(),
        )).is_ok()
    }

    async fn disconnect_wifi(&self) -> bool {
        self.cmd_tx.try_send(NetworkCmd::DisconnectWifi).is_ok()
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
