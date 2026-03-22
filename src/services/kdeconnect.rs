use futures_util::{StreamExt, stream::SelectAll};
use zbus::{Connection, Proxy, proxy};
use zbus::proxy::{Builder as ProxyBuilder, SignalStream};
use zbus::zvariant::ObjectPath;
use async_channel::{Sender, bounded};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use super::Service;
use crate::store::ServiceStore;
use log::{error, info, warn};

const SERVICE: &str = "org.kde.kdeconnect";
const BASE: &str = "/modules/kdeconnect";

// --- D-Bus Proxy ---

#[proxy(
    interface = "org.kde.kdeconnect.daemon",
    default_service = "org.kde.kdeconnect",
    default_path = "/modules/kdeconnect"
)]
trait Daemon {
    fn devices(&self) -> zbus::Result<Vec<String>>;
    #[zbus(signal, name = "deviceAdded")]
    fn device_added(&self, id: String) -> zbus::Result<()>;
    #[zbus(signal, name = "deviceRemoved")]
    fn device_removed(&self, id: String) -> zbus::Result<()>;
}

// --- Data Types ---

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KdeConnectDeviceData {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub is_reachable: bool,
    pub is_paired: bool,
    pub battery_level: Option<i32>,
    pub battery_charging: bool,
    pub has_battery: bool,
    pub has_ping: bool,
    pub has_findmyphone: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KdeConnectData {
    pub available: bool,
    pub devices: Vec<KdeConnectDeviceData>,
}

pub enum KdeConnectCmd {
    Ping { device_id: String },
    Ring { device_id: String },
    Pair { device_id: String },
    Unpair { device_id: String },
}

// --- Service ---

pub struct KdeConnectService;

impl Service for KdeConnectService {
    type Data = KdeConnectData;
    type Cmd = KdeConnectCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(10);
        let (cmd_tx, cmd_rx) = bounded(10);

        tokio::spawn(async move {
            let mut daemon_child: Option<Child> = None;

            if !Self::daemon_running().await {
                info!("[kdeconnect] Daemon not running, starting kdeconnectd...");
                Self::start_daemon(&mut daemon_child);
                for _ in 0..10 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if Self::daemon_running().await {
                        info!("[kdeconnect] Daemon is now running");
                        break;
                    }
                }
            }

            let connection = loop {
                match Connection::session().await {
                    Ok(conn) => break conn,
                    Err(e) => error!("[kdeconnect] Failed to connect to session bus: {e}"),
                }
                tokio::time::sleep(Duration::from_secs(3)).await;
            };
            info!("[kdeconnect] Connected to session bus");

            let daemon = DaemonProxy::new(&connection).await.unwrap();
            let mut device_added = daemon.receive_device_added().await.unwrap();
            let mut device_removed = daemon.receive_device_removed().await.unwrap();

            let mut cmd_rx = Box::pin(cmd_rx);
            let mut devices: HashMap<String, KdeConnectDeviceData> = HashMap::new();
            // Combined stream of all device property signals
            let mut signal_stream = SelectAll::new();
            let mut current_data = KdeConnectData::default();

            // Initial fetch
            Self::refresh_all(&connection, &daemon, &mut devices, &mut signal_stream).await;
            Self::emit_sorted(&devices, &mut current_data, &data_tx).await;

            loop {
                // Poll all signal streams to find which device changed
                let mut changed_device_id: Option<String> = None;

                tokio::select! {
                    Some(signal) = device_added.next() => {
                        if let Ok(args) = signal.args() {
                            let id = args.id;
                            info!("[kdeconnect] Device added: {id}");
                            Self::add_device(&connection, &id, &mut devices, &mut signal_stream).await;
                            Self::emit_sorted(&devices, &mut current_data, &data_tx).await;
                        }
                        continue;
                    }
                    Some(signal) = device_removed.next() => {
                        if let Ok(args) = signal.args() {
                            let id = args.id;
                            info!("[kdeconnect] Device removed: {id}");
                            devices.remove(&id);
                            Self::emit_sorted(&devices, &mut current_data, &data_tx).await;
                        }
                        continue;
                    }
                    Some(msg) = signal_stream.next() => {
                        // Extract device ID from path
                        if let Some(path) = msg.header().path() {
                            let path_str = path.as_str();
                            // Path format: /modules/kdeconnect/devices/{id} or /modules/kdeconnect/devices/{id}/battery
                            if let Some(rest) = path_str.strip_prefix(&format!("{BASE}/devices/")) {
                                let device_id = rest.split('/').next().unwrap_or("").to_string();
                                if !device_id.is_empty() {
                                    // Parse the signal
                                    if let Some(member) = msg.header().member() {
                                        let member_str = member.to_string();
                                        Self::process_signal(&msg, &member_str, &device_id, &mut devices);
                                    }
                                    changed_device_id = Some(device_id);
                                }
                            }
                        }
                    }
                    Some(cmd) = cmd_rx.next() => {
                        Self::handle_cmd(&connection, cmd).await;
                        continue;
                    }
                    else => break,
                }

                // If a device signal changed, emit updated data
                if changed_device_id.is_some() {
                    Self::emit_sorted(&devices, &mut current_data, &data_tx).await;
                }
            }
        });

        (ServiceStore::new(data_rx, Default::default()), cmd_tx)
    }
}

impl KdeConnectService {
    fn check_available() -> bool {
        Command::new("which")
            .arg("kdeconnectd")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    async fn daemon_running() -> bool {
        if let Ok(conn) = Connection::session().await {
            if let Ok(reply) = conn.call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "NameHasOwner",
                &("org.kde.kdeconnect",),
            ).await {
                if let Ok(has_owner) = reply.body().deserialize::<bool>() {
                    return has_owner;
                }
            }
        }
        false
    }

    fn start_daemon(child: &mut Option<Child>) {
        if child.is_some() { return; }
        if !Self::check_available() {
            warn!("[kdeconnect] kdeconnectd not found in PATH");
            return;
        }
        match Command::new("kdeconnectd")
            .env("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/run/dbus/system_bus_socket")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => {
                info!("[kdeconnect] Started kdeconnectd (pid {})", c.id());
                *child = Some(c);
            }
            Err(e) => error!("[kdeconnect] Failed to start kdeconnectd: {e}"),
        }
    }

    async fn refresh_all(
        connection: &Connection,
        daemon: &DaemonProxy<'_>,
        devices: &mut HashMap<String, KdeConnectDeviceData>,
        signal_stream: &mut SelectAll<SignalStream<'static>>,
    ) {
        let device_ids = daemon.devices().await.unwrap_or_default();
        for id in &device_ids {
            Self::add_device(connection, id, devices, signal_stream).await;
        }
    }

    async fn add_device(
        connection: &Connection,
        id: &str,
        devices: &mut HashMap<String, KdeConnectDeviceData>,
        signal_stream: &mut SelectAll<SignalStream<'static>>,
    ) {
        if let Some(dev) = Self::fetch_device(connection, id).await {
            devices.insert(id.to_string(), dev);

            // Subscribe to device signals: nameChanged, pairStateChanged, reachableChanged
            let dev_path = format!("{BASE}/devices/{id}");
            if let Ok(proxy) = Self::make_proxy(connection, dev_path.clone(), "org.kde.kdeconnect.device".to_string()).await {
                for signal in ["nameChanged", "pairStateChanged", "reachableChanged"] {
                    if let Ok(stream) = proxy.receive_signal(signal).await {
                        signal_stream.push(stream);
                    }
                }
            }

            // Subscribe to battery signal if plugin exists
            if devices.get(id).map_or(false, |d| d.has_battery) {
                let bat_path = format!("{BASE}/devices/{id}/battery");
                if let Ok(proxy) = Self::make_proxy(connection, bat_path.clone(), "org.kde.kdeconnect.device.battery".to_string()).await {
                    if let Ok(stream) = proxy.receive_signal("refreshed").await {
                        signal_stream.push(stream);
                    }
                }
            }
        }
    }

    async fn fetch_device(connection: &Connection, id: &str) -> Option<KdeConnectDeviceData> {
        let path = format!("{BASE}/devices/{id}");

        // GetAll device properties in one call (includes supportedPlugins!)
        let props = Self::get_all(connection, &path, "org.kde.kdeconnect.device").await?;

        let name = props.get("name")
            .and_then(|v| <String>::try_from(v.clone()).ok())
            .unwrap_or_else(|| "Unknown".into());
        let device_type = props.get("type")
            .and_then(|v| <String>::try_from(v.clone()).ok())
            .unwrap_or_else(|| "unknown".into());
        let is_reachable = props.get("isReachable")
            .and_then(|v| bool::try_from(v.clone()).ok())
            .unwrap_or(false);
        let is_paired = props.get("isPaired")
            .and_then(|v| bool::try_from(v.clone()).ok())
            .unwrap_or(false);

        // supportedPlugins from GetAll
        let supported: Vec<String> = props.get("supportedPlugins")
            .and_then(|v| <Vec<String>>::try_from(v.clone()).ok())
            .unwrap_or_default();

        let has_battery = supported.iter().any(|p| p == "kdeconnect_battery");
        let has_ping = supported.iter().any(|p| p == "kdeconnect_ping");
        let has_findmyphone = supported.iter().any(|p| p == "kdeconnect_findmyphone");

        // Battery: one GetAll call
        let (battery_level, battery_charging) = if has_battery {
            let bat_path = format!("{path}/battery");
            if let Some(bat_props) = Self::get_all(
                connection, &bat_path, "org.kde.kdeconnect.device.battery"
            ).await {
                let level = bat_props.get("charge")
                    .and_then(|v| i32::try_from(v.clone()).ok());
                let charging = bat_props.get("isCharging")
                    .and_then(|v| bool::try_from(v.clone()).ok())
                    .unwrap_or(false);
                (level, charging)
            } else {
                (None, false)
            }
        } else {
            (None, false)
        };

        Some(KdeConnectDeviceData {
            id: id.to_string(),
            name, device_type, is_reachable, is_paired,
            battery_level, battery_charging,
            has_battery, has_ping, has_findmyphone,
        })
    }

    async fn get_all(
        connection: &Connection,
        path: &str,
        interface: &str,
    ) -> Option<HashMap<String, zbus::zvariant::OwnedValue>> {
        let reply = connection.call_method(
            Some(SERVICE),
            ObjectPath::try_from(path).ok()?,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(interface,),
        ).await.ok()?;
        reply.body().deserialize().ok()
    }

    async fn make_proxy(
        connection: &Connection,
        path: String,
        interface: String,
    ) -> zbus::Result<Proxy<'static>> {
        let obj_path = zbus::zvariant::OwnedObjectPath::try_from(path.as_str())
            .map_err(|_| zbus::Error::MissingParameter("path"))?;
        let iface = zbus::names::OwnedInterfaceName::try_from(interface.as_str())
            .map_err(|_| zbus::Error::MissingParameter("interface"))?;
        ProxyBuilder::<Proxy>::new(connection)
            .destination(SERVICE)?
            .path(obj_path)?
            .interface(iface)?
            .build()
            .await
    }

    fn process_signal(
        msg: &zbus::Message,
        member: &str,
        device_id: &str,
        devices: &mut HashMap<String, KdeConnectDeviceData>,
    ) {
        if let Some(dev) = devices.get_mut(device_id) {
            match member {
                "reachableChanged" => {
                    if let Ok((reachable,)) = msg.body().deserialize::<(bool,)>() {
                        dev.is_reachable = reachable;
                    }
                }
                "pairStateChanged" => {
                    // pairState: 0=NotPaired, 1=Requested, 2=Paired
                    if let Ok((state,)) = msg.body().deserialize::<(u32,)>() {
                        dev.is_paired = state == 2;
                    }
                }
                "nameChanged" => {
                    if let Ok((name,)) = msg.body().deserialize::<(String,)>() {
                        dev.name = name;
                    }
                }
                "refreshed" => {
                    // Battery refreshed: (isCharging, charge)
                    if let Ok((charging, charge)) = msg.body().deserialize::<(bool, i32)>() {
                        dev.battery_charging = charging;
                        dev.battery_level = Some(charge);
                    }
                }
                _ => {}
            }
        }
    }

    async fn emit_sorted(
        devices: &HashMap<String, KdeConnectDeviceData>,
        current_data: &mut KdeConnectData,
        data_tx: &Sender<KdeConnectData>,
    ) {
        let mut device_list: Vec<KdeConnectDeviceData> = devices.values().cloned().collect();
        device_list.sort_by(|a, b| {
            let a_online = a.is_paired && a.is_reachable;
            let b_online = b.is_paired && b.is_reachable;
            if a_online != b_online { return b_online.cmp(&a_online); }
            if a.is_paired != b.is_paired { return b.is_paired.cmp(&a.is_paired); }
            a.name.cmp(&b.name)
        });

        let next_data = KdeConnectData { available: true, devices: device_list };
        if next_data != *current_data {
            *current_data = next_data;
            let _ = data_tx.send(current_data.clone()).await;
        }
    }

    async fn handle_cmd(connection: &Connection, cmd: KdeConnectCmd) {
        let (path, interface, method) = match &cmd {
            KdeConnectCmd::Ping { device_id } => (
                format!("{BASE}/devices/{device_id}/ping"),
                "org.kde.kdeconnect.device.ping", "sendPing",
            ),
            KdeConnectCmd::Ring { device_id } => (
                format!("{BASE}/devices/{device_id}/findmyphone"),
                "org.kde.kdeconnect.device.findmyphone", "ring",
            ),
            KdeConnectCmd::Pair { device_id } => (
                format!("{BASE}/devices/{device_id}"),
                "org.kde.kdeconnect.device", "requestPair",
            ),
            KdeConnectCmd::Unpair { device_id } => (
                format!("{BASE}/devices/{device_id}"),
                "org.kde.kdeconnect.device", "unpair",
            ),
        };
        info!("[kdeconnect] {method} to {path}");
        if let Err(e) = connection.call_method(
            Some(SERVICE),
            ObjectPath::try_from(path.as_str()).unwrap(),
            Some(interface),
            method,
            &(),
        ).await {
            error!("[kdeconnect] {method} failed: {e}");
        }
    }
}
