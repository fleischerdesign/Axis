use futures_util::StreamExt;
use zbus::{Connection, interface, proxy, zvariant::{OwnedObjectPath, OwnedValue}};
use async_channel::{Sender, bounded, Receiver};
use tokio::sync::oneshot;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use super::{Service, ServiceConfig};
use crate::store::ServiceStore;
use log::{error, info};

const BLUETOOTH_POWER_RETRY_ATTEMPTS: u32 = 5;

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
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn icon(&self) -> zbus::Result<String>;
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
    pub pairing_request: Option<PairingRequest>,
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

pub enum BluetoothFwdCmd {
    Add(String),
    Remove(String),
}

#[derive(Debug)]
pub struct PropertyChange {
    pub device_path: String,
}

/// Spawns a per-device task for each added device path.
/// Tasks listen for `PropertiesChanged` (filtered to `Connected`) and forward to the main loop.
/// On `Remove`, the device task is cancelled — no shared state, no clear-all.
fn spawn_device_property_forwarder(
    conn: zbus::Connection,
    cmd_rx: Receiver<BluetoothFwdCmd>,
) -> Receiver<PropertyChange> {
    let (prop_tx, prop_rx) = bounded::<PropertyChange>(10);

    tokio::spawn(async move {
        let mut cancel_txs: HashMap<String, oneshot::Sender<()>> = HashMap::new();

        while let Ok(cmd) = cmd_rx.recv().await {
            match cmd {
                BluetoothFwdCmd::Remove(path) => {
                    if let Some(tx) = cancel_txs.remove(&path) {
                        let _ = tx.send(());
                    }
                }
                BluetoothFwdCmd::Add(path) => {
                    if let Some(tx) = cancel_txs.remove(&path) {
                        let _ = tx.send(());
                    }
                    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
                    cancel_txs.insert(path.clone(), cancel_tx);
                    let c = conn.clone();
                    let ptx = prop_tx.clone();
                    tokio::spawn(async move {
                        let Ok(proxy) = zbus::fdo::PropertiesProxy::builder(&c)
                            .destination("org.bluez").unwrap()
                            .path(&*path).unwrap()
                            .build().await
                        else { return };
                        let Ok(changed) = proxy.receive_properties_changed().await
                        else { return };
                        let mut s = changed;
                        tokio::select! {
                            _ = async {
                                while let Some(args) = s.next().await {
                                    if args.args().ok()
                                        .map(|a| a.changed_properties.contains_key("Connected"))
                                        .unwrap_or(true)
                                    {
                                        let _ = ptx.try_send(PropertyChange { device_path: path.clone() });
                                    }
                                }
                            } => {}
                            _ = cancel_rx => {}
                        }
                    });
                }
            }
        }
    });

    prop_rx
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

            let (fwd_tx, fwd_rx) = bounded::<BluetoothFwdCmd>(10);
            let prop_rx = spawn_device_property_forwarder(connection.clone(), fwd_rx);

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
            // When we send TogglePower, skip the next fetch_data until BlueZ confirms
            // via powered_changed. Otherwise fetch_data races and reads stale state,
            // publishing incorrect is_powered which triggers the sync bridge loop.
            let mut skip_fetch_after_toggle = false;

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
                                current_data.pairing_request = None;
                                let _ = data_tx.send(current_data.clone()).await;
                            }
                            BluetoothCmd::PairReject => {
                                if let Some(tx) = pair_resolve.take() {
                                    let _ = tx.send(AgentResponse::Reject);
                                    info!("[bluetooth] Pairing rejected");
                                }
                                current_data.pairing_request = None;
                                let _ = data_tx.send(current_data.clone()).await;
                            }
                            BluetoothCmd::TogglePower(on) => {
                                // Guard: skip if already in the desired state
                                if current_data.is_powered == on {
                                    log::debug!("[bluetooth] TogglePower({on}) ignored — already in that state");
                                } else {
                                    info!("[bluetooth] Power toggled: {}", if on { "on" } else { "off" });
                                    for attempt in 0..BLUETOOTH_POWER_RETRY_ATTEMPTS {
                                        match adapter_proxy.set_powered(on).await {
                                            Ok(()) => break,
                                            Err(e) if e.to_string().contains("Busy") => {
                                                error!("[bluetooth] Power toggle busy (attempt {}/5), retrying", attempt + 1);
                                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                            }
                                            Err(e) => {
                                                error!("[bluetooth] Failed to toggle power: {e}");
                                                break;
                                            }
                                        }
                                    }
                                    // Skip fetch_data this iteration — BlueZ will emit
                                    // powered_changed which clears this flag and triggers a
                                    // fresh fetch. Skipping prevents reading stale state that
                                    // would kick off the sync bridge feedback loop.
                                    skip_fetch_after_toggle = true;
                                    if !on { is_discovering = false; }
                                }
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
                        current_data.pairing_request = Some(req);
                        let _ = data_tx.send(current_data.clone()).await;
                    }
                    // Pairing timeout
                    _ = &mut pair_timeout => {
                        if let Some(tx) = pair_resolve.take() {
                            let _ = tx.send(AgentResponse::Reject);
                            current_data.pairing_request = None;
                            let _ = data_tx.send(current_data.clone()).await;
                        }
                        pair_timeout = Box::pin(tokio::time::sleep(Duration::MAX));
                    }
                    // Other events
                    _ = interval.tick() => {}
                    Some(_) = powered_changed.next() => {
                        // BlueZ confirmed the power change — allow fetch_data to run
                        skip_fetch_after_toggle = false;
                    }
                    Some(args) = interfaces_added.next() => {
                        if let Ok(a) = args.args() {
                            if a.interfaces_and_properties.contains_key("org.bluez.Device1") {
                                let _ = fwd_tx.try_send(BluetoothFwdCmd::Add(a.object_path.to_string()));
                            }
                        }
                    }
                    Some(args) = interfaces_removed.next() => {
                        if let Ok(a) = args.args() {
                            if a.interfaces.iter().any(|i| i == "org.bluez.Device1") {
                                let _ = fwd_tx.try_send(BluetoothFwdCmd::Remove(a.object_path.to_string()));
                            }
                        }
                    }
                    Some(chg) = prop_rx.next() => {
                        if let Some(updated) = Self::fetch_single_device(&connection, &chg.device_path).await {
                            if Self::update_device_in_list(&mut current_data, updated) {
                                let _ = data_tx.send(current_data.clone()).await;
                            }
                        }
                    }
                }

                // Scan state change
                let is_new_scan = is_discovering && !was_discovering;
                let is_scan_ended = !is_discovering && was_discovering;
                if is_new_scan || is_scan_ended {
                    known_devices.clear();
                }
                was_discovering = is_discovering;

                // Skip fetch while a TogglePower is in-flight. BlueZ's
                // powered_changed event will clear this flag so the next
                // iteration picks up the confirmed state.
                if !skip_fetch_after_toggle {
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
            }
        });

        (ServiceStore::new(data_rx, Default::default()), cmd_tx)
    }
}

impl ServiceConfig for BluetoothService {
    fn get_enabled(data: &BluetoothData) -> bool { data.is_powered }
    fn cmd_set_enabled(on: bool) -> BluetoothCmd { BluetoothCmd::TogglePower(on) }
}

impl BluetoothService {
    async fn fetch_single_device(
        conn: &zbus::Connection,
        device_path: &str,
    ) -> Option<BluetoothDeviceData> {
        let path = OwnedObjectPath::try_from(device_path).ok()?;
        let proxy = BluetoothDeviceProxy::builder(conn)
            .path(path).ok()?
            .build().await.ok()?;

        let name = proxy.name().await
            .unwrap_or_else(|_| "Unknown Device".to_string());
        let is_connected = proxy.connected().await.unwrap_or(false);
        let is_paired = proxy.paired().await.unwrap_or(false);
        let icon = proxy.icon().await
            .unwrap_or_else(|_| "bluetooth-symbolic".to_string());

        Some(BluetoothDeviceData {
            name, is_connected, is_paired,
            path: device_path.to_string(), icon,
            first_seen: None,
        })
    }

    fn update_device_in_list(data: &mut BluetoothData, updated: BluetoothDeviceData) -> bool {
        if let Some(existing) = data.devices.iter_mut().find(|d| d.path == updated.path) {
            if *existing != updated {
                let first_seen = existing.first_seen;
                *existing = updated;
                existing.first_seen = first_seen;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

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
                    .then_with(|| a.path.cmp(&b.path))
                });
            }
        } else if !is_powered {
            devices.clear();
        }

        BluetoothData {
            is_powered,
            devices,
            pairing_request: old_data.pairing_request.clone(),
        }
    }
}
