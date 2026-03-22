use futures_util::StreamExt;
use zbus::{proxy, Connection, zvariant::{OwnedObjectPath, OwnedValue}};
use async_channel::{Sender, bounded};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use super::Service;
use crate::store::ServiceStore;
use log::{error, info};

#[proxy(
    interface = "org.bluez.Adapter1",
    default_service = "org.bluez",
    default_path = "/org/bluez/hci0"
)]
trait BluetoothAdapter {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
    fn start_discovery(&self) -> zbus::Result<()>;
    fn stop_discovery(&self) -> zbus::Result<()>;
}

#[proxy(interface = "org.bluez.Device1", default_service = "org.bluez")]
trait BluetoothDevice {
    fn connect(&self) -> zbus::Result<()>;
    fn disconnect(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.DBus.ObjectManager",
    default_service = "org.bluez",
    default_path = "/"
)]
trait ObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>>;
    #[zbus(signal)]
    fn interfaces_added(&self, object_path: OwnedObjectPath, interfaces_and_properties: HashMap<String, HashMap<String, OwnedValue>>) -> zbus::Result<()>;
    #[zbus(signal)]
    fn interfaces_removed(&self, object_path: OwnedObjectPath, interfaces: Vec<String>) -> zbus::Result<()>;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BluetoothDeviceData {
    pub name: String,
    pub is_connected: bool,
    pub is_paired: bool,
    pub path: String,
    pub icon: String,
    pub first_seen: Option<SystemTime>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BluetoothData {
    pub is_powered: bool,
    pub devices: Vec<BluetoothDeviceData>,
}

pub enum BluetoothCmd {
    TogglePower(bool),
    Connect(String),
    Disconnect(String),
    Scan,
    StopScan,
}

pub struct BluetoothService;

impl Service for BluetoothService {
    type Data = BluetoothData;
    type Cmd = BluetoothCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(10);
        let (cmd_tx, cmd_rx) = bounded(10);

        tokio::spawn(async move {
            // Retry connection setup with backoff
            let (connection, adapter_proxy, obj_manager) = loop {
                match Connection::system().await {
                    Ok(conn) => {
                        let adapter = BluetoothAdapterProxy::new(&conn).await;
                        let obj_mgr = ObjectManagerProxy::new(&conn).await;
                        if let (Ok(a), Ok(o)) = (adapter, obj_mgr) {
                            break (conn, a, o);
                        }
                        error!("[bluetooth] Failed to create proxies, retrying...");
                    }
                    Err(e) => error!("[bluetooth] Failed to connect to D-Bus: {e}"),
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            };
            info!("[bluetooth] Connected to system bus");

            let mut powered_changed = adapter_proxy.receive_powered_changed().await;
            let mut interfaces_added = obj_manager.receive_interfaces_added().await.unwrap();
            let mut interfaces_removed = obj_manager.receive_interfaces_removed().await.unwrap();
            let mut interval = tokio::time::interval(Duration::from_secs(60)); 

            let mut cmd_rx = Box::pin(cmd_rx);
            let mut current_data = BluetoothData::default();
            let mut is_discovering = false;
            let mut was_discovering = false;
            let mut known_devices: HashMap<String, SystemTime> = HashMap::new();

            loop {

                tokio::select! {
                    _ = interval.tick() => {}
                    Some(_) = powered_changed.next() => {}
                    Some(_) = interfaces_added.next() => {}
                    Some(_) = interfaces_removed.next() => {}
                    Some(cmd) = cmd_rx.next() => {
                        match cmd {
                            BluetoothCmd::TogglePower(on) => {
                                info!("[bluetooth] Power toggled: {}", if on { "on" } else { "off" });
                                if let Err(e) = adapter_proxy.set_powered(on).await {
                                    error!("[bluetooth] Failed to toggle power: {e}");
                                }
                                if !on { is_discovering = false; }
                            }
                            BluetoothCmd::Scan => {
                                info!("[bluetooth] Discovery started");
                                if let Err(e) = adapter_proxy.start_discovery().await {
                                    error!("[bluetooth] Failed to start discovery: {e}");
                                }
                                is_discovering = true;
                            }
                            BluetoothCmd::StopScan => {
                                info!("[bluetooth] Discovery stopped");
                                if let Err(e) = adapter_proxy.stop_discovery().await {
                                    error!("[bluetooth] Failed to stop discovery: {e}");
                                }
                                is_discovering = false;
                            }
                            BluetoothCmd::Connect(path_str) => {
                                info!("[bluetooth] Connecting device");
                                if let Ok(path) = OwnedObjectPath::try_from(path_str) {
                                    if let Ok(dev_proxy) = BluetoothDeviceProxy::builder(&connection).path(path).unwrap().build().await {
                                        if let Err(e) = dev_proxy.connect().await {
                                            error!("[bluetooth] Failed to connect device: {e}");
                                        }
                                    }
                                }
                            }
                            BluetoothCmd::Disconnect(path_str) => {
                                info!("[bluetooth] Disconnecting device");
                                if let Ok(path) = OwnedObjectPath::try_from(path_str) {
                                    if let Ok(dev_proxy) = BluetoothDeviceProxy::builder(&connection).path(path).unwrap().build().await {
                                        if let Err(e) = dev_proxy.disconnect().await {
                                            error!("[bluetooth] Failed to disconnect device: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Prüfen ob ein neuer Scan gestartet oder beendet wurde
                let is_new_scan = is_discovering && !was_discovering;
                let is_scan_ended = !is_discovering && was_discovering;
                
                if is_new_scan || is_scan_ended {
                    known_devices.clear();
                }
                was_discovering = is_discovering;

                let next_data = Self::fetch_data(&adapter_proxy, &obj_manager, is_discovering, &current_data, &mut known_devices, is_new_scan).await;
                if next_data != current_data {
                    current_data = next_data;
                    let _ = data_tx.send(current_data.clone()).await;
                }
            }
        });

        (ServiceStore::new(data_rx, Default::default()), cmd_tx)
    }
}

impl BluetoothService {

    async fn fetch_data(adapter: &BluetoothAdapterProxy<'_>, obj_manager: &ObjectManagerProxy<'_>, include_devices: bool, old_data: &BluetoothData, known_devices: &mut HashMap<String, SystemTime>, is_new_scan: bool) -> BluetoothData {
        let is_powered = adapter.powered().await.unwrap_or(false);
        // Wenn BT aus ist, brauchen wir keine Geräte-Abfrage
        let actual_include = include_devices && is_powered;
        
        let mut devices = if actual_include { Vec::new() } else { old_data.devices.clone() };

        if actual_include {
            if let Ok(objects) = obj_manager.get_managed_objects().await {
                for (path, interfaces) in objects {
                    if let Some(props) = interfaces.get("org.bluez.Device1") {
                        let name = props.get("Name")
                            .or_else(|| props.get("Alias"))
                            .and_then(|v| <&str>::try_from(v).ok())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "Unknown Device".to_string());
                        
                        let is_connected = props.get("Connected")
                            .and_then(|v| bool::try_from(v).ok())
                            .unwrap_or(false);

                        let is_paired = props.get("Paired")
                            .and_then(|v| bool::try_from(v).ok())
                            .unwrap_or(false);

                        let icon = props.get("Icon")
                            .and_then(|v| <&str>::try_from(v).ok())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "bluetooth-symbolic".to_string());

                        let path_str = path.to_string();
                        
                        // Timestamp für ungekoppelte Geräte
                        let first_seen = if !is_connected && !is_paired {
                            if is_new_scan {
                                // Bei neuem Scan: Timestamp zurücksetzen und neu setzen
                                let now = SystemTime::now();
                                known_devices.insert(path_str.clone(), now);
                                Some(now)
                            } else {
                                // Bekanntes Gerät: alten Timestamp behalten, oder neu setzen
                                Some(*known_devices.entry(path_str.clone()).or_insert_with(SystemTime::now))
                            }
                        } else {
                            None
                        };

                        devices.push(BluetoothDeviceData {
                            name,
                            is_connected,
                            is_paired,
                            path: path_str,
                            icon,
                            first_seen,
                        });
                    }
                }
                devices.sort_by(|a, b| {
                    // Verbundene Geräte zuerst
                    if a.is_connected != b.is_connected {
                        return b.is_connected.cmp(&a.is_connected);
                    }
                    // Gekoppelte Geräte danach
                    if a.is_paired != b.is_paired {
                        return b.is_paired.cmp(&a.is_paired);
                    }
                    // Ungekoppelte Geräte nach Timestamp sortieren (älteste zuerst)
                    match (&a.first_seen, &b.first_seen) {
                        (Some(a_time), Some(b_time)) => a_time.cmp(b_time),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
        } else if !is_powered {
            devices.clear();
        }

        BluetoothData { is_powered, devices }
    }
}
