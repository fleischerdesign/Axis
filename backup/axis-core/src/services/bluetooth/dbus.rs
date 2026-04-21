use async_channel::Sender;
use serde::{Deserialize, Serialize};
use zbus::interface;
use zbus::object_server::SignalEmitter;

use super::{BluetoothData, BluetoothDeviceData, BluetoothCmd};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbusBluetoothDevice {
    pub path: String,
    pub name: String,
    pub icon: String,
    pub is_connected: bool,
    pub is_paired: bool,
}

impl From<&BluetoothDeviceData> for DbusBluetoothDevice {
    fn from(d: &BluetoothDeviceData) -> Self {
        Self {
            path: d.path.clone(),
            name: d.name.clone(),
            icon: d.icon.clone(),
            is_connected: d.is_connected,
            is_paired: d.is_paired,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BluetoothStateSnapshot {
    pub enabled: bool,
    pub devices: Vec<DbusBluetoothDevice>,
    pub pairing_device: Option<String>,
    pub pairing_name: Option<String>,
}

impl Default for BluetoothStateSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            devices: Vec::new(),
            pairing_device: None,
            pairing_name: None,
        }
    }
}

pub fn build_snapshot(data: &BluetoothData) -> BluetoothStateSnapshot {
    BluetoothStateSnapshot {
        enabled: data.is_powered,
        devices: data.devices.iter().map(DbusBluetoothDevice::from).collect(),
        pairing_device: data.pairing_request.as_ref().map(|r| r.device_path.clone()),
        pairing_name: data.pairing_request.as_ref().map(|r| r.device_name.clone()),
    }
}

pub struct BluetoothDbusServer {
    cmd_tx: Sender<BluetoothCmd>,
    state_rx: tokio::sync::watch::Receiver<BluetoothStateSnapshot>,
}

impl BluetoothDbusServer {
    pub fn new(cmd_tx: Sender<BluetoothCmd>, state_rx: tokio::sync::watch::Receiver<BluetoothStateSnapshot>) -> Self {
        Self { cmd_tx, state_rx }
    }
}

#[interface(name = "org.axis.Shell.Bluetooth")]
impl BluetoothDbusServer {
    async fn get_state(&self) -> String {
        serde_json::to_string(&*self.state_rx.borrow()).unwrap_or_default()
    }

    async fn set_enabled(&self, enabled: bool) -> bool {
        self.cmd_tx.try_send(BluetoothCmd::TogglePower(enabled)).is_ok()
    }

    async fn connect_device(&self, path: &str) -> bool {
        self.cmd_tx.try_send(BluetoothCmd::Connect(path.to_string())).is_ok()
    }

    async fn disconnect_device(&self, path: &str) -> bool {
        self.cmd_tx.try_send(BluetoothCmd::Disconnect(path.to_string())).is_ok()
    }

    async fn start_scan(&self) -> bool {
        self.cmd_tx.try_send(BluetoothCmd::Scan).is_ok()
    }

    async fn stop_scan(&self) -> bool {
        self.cmd_tx.try_send(BluetoothCmd::StopScan).is_ok()
    }

    async fn accept_pairing(&self) -> bool {
        self.cmd_tx.try_send(BluetoothCmd::PairAccept).is_ok()
    }

    async fn reject_pairing(&self) -> bool {
        self.cmd_tx.try_send(BluetoothCmd::PairReject).is_ok()
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
