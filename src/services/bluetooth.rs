use futures_util::StreamExt;
use futures_util::stream::select_all;
use zbus::{Connection, interface, proxy, zvariant::{OwnedObjectPath, OwnedValue}};
use async_channel::{Sender, bounded, Receiver};
use tokio::sync::oneshot;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime};
use super::Service;
use crate::store::ServiceStore;
use log::{error, info};

// ─── D-Bus Proxies ──────────────────────────────────────────────────────────

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

#[proxy(
    interface = "org.bluez.AgentManager1",
    default_service = "org.bluez",
    default_path = "/org/bluez"
)]
trait AgentManager {
    fn register_agent(&self, agent: &OwnedObjectPath, capability: &str) -> zbus::Result<()>;
    fn unregister_agent(&self, agent: &OwnedObjectPath) -> zbus::Result<()>;
    fn request_default_agent(&self, agent: &OwnedObjectPath) -> zbus::Result<()>;
}

// ─── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PairingType {
    Confirmation,
    PinCode,
    Passkey,
    Authorization,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairingRequest {
    pub device_path: String,
    pub device_name: String,
    pub passkey: Option<String>,
    pub pairing_type: PairingType,
}

#[derive(Debug, Clone)]
pub enum AgentResponse {
    Accept(String),
    Reject,
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
    PairAccept,
    PairReject,
}

// ─── Properties Changed Forwarding ───────────────────────────────────────────

#[derive(Debug)]
pub struct PropertyChange;

/// Monitors all BlueZ device paths for property changes.
/// Re-scans managed objects every 5s to discover new devices.
/// Forwards all `PropertiesChanged` signals through the channel.
fn spawn_device_property_monitor(conn: zbus::Connection) -> Receiver<PropertyChange> {
    let (tx, rx) = bounded::<PropertyChange>(10);

    tokio::spawn(async move {
        let mut streams = select_all(Vec::<futures_util::stream::BoxStream<()>>::new());
        let mut known: HashSet<String> = HashSet::new();
        let mut rescan = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                Some(()) = streams.next() => {
                    let _ = tx.try_send(PropertyChange);
                }
                _ = rescan.tick() => {
                    let Some(obj_mgr) = ObjectManagerProxy::new(&conn).await.ok() else { continue };
                    let Ok(objects) = obj_mgr.get_managed_objects().await else { continue };

                    for (path, interfaces) in &objects {
                        if !interfaces.contains_key("org.bluez.Device1") {
                            continue;
                        }
                        let path_str = path.to_string();
                        if !known.insert(path_str.clone()) {
                            continue;
                        }
                        if let Ok(proxy) = zbus::fdo::PropertiesProxy::builder(&conn)
                            .destination("org.bluez").unwrap()
                            .path(path).unwrap()
                            .build().await
                        {
                            match proxy.receive_properties_changed().await {
                                Ok(s) => streams.push(s.map(|_| ()).boxed()),
                                Err(e) => {
                                    error!("[bluetooth] Failed to subscribe {path_str}: {e}");
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    rx
}

// ─── Global Pairing State (bypasses async_channel for instant GTK delivery) ─

static PAIRING_STATE: std::sync::LazyLock<Arc<Mutex<Option<PairingRequest>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));
static PAIRING_NOTIF_ID: AtomicU32 = AtomicU32::new(0);

pub struct PairingUiState {
    pub request: Option<PairingRequest>,
    pub should_close_notif: bool,
    pub notif_id: u32,
}

pub fn get_pairing_ui_state() -> PairingUiState {
    let request = PAIRING_STATE.lock().unwrap().clone();
    let notif_id = PAIRING_NOTIF_ID.load(Ordering::SeqCst);
    PairingUiState {
        should_close_notif: request.is_none() && notif_id > 0,
        notif_id,
        request,
    }
}

pub fn set_pairing_notification_id(id: u32) {
    PAIRING_NOTIF_ID.store(id, Ordering::SeqCst);
}

fn set_pairing_request(req: Option<PairingRequest>) {
    *PAIRING_STATE.lock().unwrap() = req;
}

// ─── BlueZ Agent ────────────────────────────────────────────────────────────

struct BluetoothAgent {
    req_tx: Sender<(PairingRequest, oneshot::Sender<AgentResponse>)>,
    conn: zbus::Connection,
}

const AGENT_PATH: &str = "/org/axis/bluetooth_agent";

impl BluetoothAgent {
    fn reject() -> zbus::fdo::Error {
        zbus::fdo::Error::Failed("Rejected".to_string())
    }

    async fn wait_for_response(
        &self,
        req: PairingRequest,
    ) -> Result<String, zbus::fdo::Error> {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.req_tx.send((req, resp_tx)).await.is_err() {
            return Err(Self::reject());
        }
        tokio::time::timeout(Duration::from_secs(30), resp_rx)
            .await
            .map_err(|_| Self::reject())?
            .map_err(|_| Self::reject())
            .and_then(|r| match r {
                AgentResponse::Accept(s) => Ok(s),
                AgentResponse::Reject => Err(Self::reject()),
            })
    }

    async fn resolve_device_name(&self, path: &OwnedObjectPath) -> String {
        let proxy = ObjectManagerProxy::new(&self.conn).await.ok();
        if let Some(obj_mgr) = proxy {
            if let Ok(objects) = obj_mgr.get_managed_objects().await {
                if let Some(interfaces) = objects.get(path) {
                    if let Some(props) = interfaces.get("org.bluez.Device1") {
                        return props.get("Name")
                            .or_else(|| props.get("Alias"))
                            .and_then(|v| <&str>::try_from(v).ok())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| path.to_string());
                    }
                }
            }
        }
        path.to_string()
    }
}

#[interface(name = "org.bluez.Agent1")]
impl BluetoothAgent {
    async fn release(&self) {
        info!("[bluetooth] Agent released");
    }

    async fn request_pin_code(&self, device: OwnedObjectPath) -> Result<String, zbus::fdo::Error> {
        info!("[bluetooth] RequestPinCode: {device}");
        let name = self.resolve_device_name(&device).await;
        let req = PairingRequest {
            device_path: device.to_string(),
            device_name: name,
            passkey: None,
            pairing_type: PairingType::PinCode,
        };
        self.wait_for_response(req).await
    }

    async fn display_pin_code(&self, device: OwnedObjectPath, pincode: String) {
        info!("[bluetooth] DisplayPinCode: {device} → {pincode}");
    }

    async fn request_passkey(&self, device: OwnedObjectPath) -> Result<u32, zbus::fdo::Error> {
        info!("[bluetooth] RequestPasskey: {device}");
        let name = self.resolve_device_name(&device).await;
        let req = PairingRequest {
            device_path: device.to_string(),
            device_name: name,
            passkey: None,
            pairing_type: PairingType::Passkey,
        };
        let value = self.wait_for_response(req).await?;
        value.parse::<u32>().map_err(|_| Self::reject())
    }

    async fn display_passkey(&self, device: OwnedObjectPath, passkey: u32, entered: u16) {
        info!("[bluetooth] DisplayPasskey: {device} → {passkey} ({entered} entered)");
    }

    async fn request_confirmation(
        &self,
        device: OwnedObjectPath,
        passkey: u32,
    ) -> Result<(), zbus::fdo::Error> {
        info!("[bluetooth] RequestConfirmation: {device} → {passkey}");
        let name = self.resolve_device_name(&device).await;
        let req = PairingRequest {
            device_path: device.to_string(),
            device_name: name,
            passkey: Some(format!("{passkey:06}")),
            pairing_type: PairingType::Confirmation,
        };
        self.wait_for_response(req).await.map(|_| ())
    }

    async fn request_authorization(&self, device: OwnedObjectPath) -> Result<(), zbus::fdo::Error> {
        info!("[bluetooth] RequestAuthorization: {device}");
        let name = self.resolve_device_name(&device).await;
        let req = PairingRequest {
            device_path: device.to_string(),
            device_name: name,
            passkey: None,
            pairing_type: PairingType::Authorization,
        };
        self.wait_for_response(req).await.map(|_| ())
    }

    async fn authorize_service(&self, _device: OwnedObjectPath, _uuid: String) -> Result<(), zbus::fdo::Error> {
        info!("[bluetooth] AuthorizeService: {_device} / {_uuid} — auto-accepted");
        Ok(())
    }

    fn cancel(&self) {
        info!("[bluetooth] Agent cancelled");
    }
}

// ─── Service ────────────────────────────────────────────────────────────────

pub struct BluetoothService;

impl Service for BluetoothService {
    type Data = BluetoothData;
    type Cmd = BluetoothCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(10);
        let (cmd_tx, cmd_rx) = bounded(10);

        tokio::spawn(async move {
            // --- Setup with retry ---
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

            // --- Register BlueZ agent ---
            let (pair_tx, pair_rx): (
                Sender<(PairingRequest, oneshot::Sender<AgentResponse>)>,
                Receiver<(PairingRequest, oneshot::Sender<AgentResponse>)>,
            ) = bounded(1);

            loop {
                let agent = BluetoothAgent {
                    req_tx: pair_tx.clone(),
                    conn: connection.clone(),
                };
                match connection.object_server().at(AGENT_PATH, agent).await {
                    Ok(true) => {
                        let agent_path = OwnedObjectPath::try_from(AGENT_PATH).unwrap();
                        let agent_mgr = AgentManagerProxy::builder(&connection)
                            .path("/org/bluez").unwrap()
                            .build().await.unwrap();

                        if let Err(e) = agent_mgr.register_agent(&agent_path, "KeyboardDisplay").await {
                            error!("[bluetooth] Agent registration failed: {e}");
                        } else if let Err(e) = agent_mgr.request_default_agent(&agent_path).await {
                            error!("[bluetooth] Default agent request failed: {e}");
                        } else {
                            info!("[bluetooth] Agent registered as default");
                            break;
                        }
                    }
                    Ok(false) => error!("[bluetooth] Agent interface already registered"),
                    Err(e) => error!("[bluetooth] Failed to register agent: {e}"),
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            let prop_rx = spawn_device_property_monitor(connection.clone());

            // --- Event streams ---
            let mut powered_changed = adapter_proxy.receive_powered_changed().await;
            let mut interfaces_added = obj_manager.receive_interfaces_added().await.unwrap();
            let mut interfaces_removed = obj_manager.receive_interfaces_removed().await.unwrap();
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            let mut cmd_rx = Box::pin(cmd_rx);
            let mut current_data = BluetoothData::default();
            let mut is_discovering = false;
            let mut was_discovering = false;
            let mut known_devices: HashMap<String, SystemTime> = HashMap::new();
            let mut pair_rx = Box::pin(pair_rx);
            let mut prop_rx = Box::pin(prop_rx);
            let mut pair_resolve: Option<oneshot::Sender<AgentResponse>> = None;
            let mut pair_timeout = Box::pin(tokio::time::sleep(Duration::MAX));

            loop {
                tokio::select! {
                    biased;
                    // UI accept/reject
                    Some(cmd) = cmd_rx.next() => {
                        match cmd {
                            BluetoothCmd::PairAccept => {
                                if let Some(tx) = pair_resolve.take() {
                                    let _ = tx.send(AgentResponse::Accept(String::new()));
                                    info!("[bluetooth] Pairing accepted");
                                }
                                set_pairing_request(None);
                            }
                            BluetoothCmd::PairReject => {
                                if let Some(tx) = pair_resolve.take() {
                                    let _ = tx.send(AgentResponse::Reject);
                                    info!("[bluetooth] Pairing rejected");
                                }
                                set_pairing_request(None);
                            }
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
                                let conn = connection.clone();
                                tokio::spawn(async move {
                                    if let Ok(path) = OwnedObjectPath::try_from(path_str) {
                                        if let Ok(dev_proxy) = BluetoothDeviceProxy::builder(&conn).path(path).unwrap().build().await {
                                            if let Err(e) = dev_proxy.connect().await {
                                                error!("[bluetooth] Failed to connect device: {e}");
                                            }
                                        }
                                    }
                                });
                            }
                            BluetoothCmd::Disconnect(path_str) => {
                                info!("[bluetooth] Disconnecting device");
                                let conn = connection.clone();
                                tokio::spawn(async move {
                                    if let Ok(path) = OwnedObjectPath::try_from(path_str) {
                                        if let Ok(dev_proxy) = BluetoothDeviceProxy::builder(&conn).path(path).unwrap().build().await {
                                            if let Err(e) = dev_proxy.disconnect().await {
                                                error!("[bluetooth] Failed to disconnect device: {e}");
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    // Agent pairing request
                    Some((req, resp_tx)) = pair_rx.next() => {
                        pair_resolve = Some(resp_tx);
                        pair_timeout = Box::pin(tokio::time::sleep(Duration::from_secs(30)));
                        set_pairing_request(Some(req));
                    }
                    // Pairing timeout
                    _ = &mut pair_timeout => {
                        if let Some(tx) = pair_resolve.take() {
                            let _ = tx.send(AgentResponse::Reject);
                            set_pairing_request(None);
                        }
                    }
                    // Other events
                    _ = interval.tick() => {}
                    Some(_) = powered_changed.next() => {}
                    Some(_) = interfaces_added.next() => {}
                    Some(_) = interfaces_removed.next() => {}
                    Some(_) = prop_rx.next() => {}
                }

                // Scan state change
                let is_new_scan = is_discovering && !was_discovering;
                let is_scan_ended = !is_discovering && was_discovering;
                if is_new_scan || is_scan_ended {
                    known_devices.clear();
                }
                was_discovering = is_discovering;

                let next_data = Self::fetch_data(
                    &adapter_proxy, &obj_manager,
                    is_discovering, &current_data,
                    &mut known_devices, is_new_scan,
                ).await;

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
    async fn fetch_data(
        adapter: &BluetoothAdapterProxy<'_>,
        obj_manager: &ObjectManagerProxy<'_>,
        include_devices: bool,
        old_data: &BluetoothData,
        known_devices: &mut HashMap<String, SystemTime>,
        is_new_scan: bool,
    ) -> BluetoothData {
        let is_powered = adapter.powered().await.unwrap_or(false);
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

                        let first_seen = if !is_connected && !is_paired {
                            if is_new_scan {
                                let now = SystemTime::now();
                                known_devices.insert(path_str.clone(), now);
                                Some(now)
                            } else {
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
                    if a.is_connected != b.is_connected {
                        return b.is_connected.cmp(&a.is_connected);
                    }
                    if a.is_paired != b.is_paired {
                        return b.is_paired.cmp(&a.is_paired);
                    }
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

        BluetoothData {
            is_powered,
            devices,
        }
    }
}
