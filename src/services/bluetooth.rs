use futures_util::StreamExt;
use zbus::{proxy, Connection, zvariant::{OwnedObjectPath, OwnedValue, Type}};
use futures_channel::mpsc;
use std::collections::HashMap;

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

#[proxy(
    interface = "org.bluez.Device1",
    default_service = "org.bluez"
)]
trait BluetoothDevice {
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn address(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn icon(&self) -> zbus::Result<String>;

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

#[derive(Debug, Clone, Default, Type, PartialEq)]
pub struct BluetoothDeviceData {
    pub name: String,
    pub is_connected: bool,
    pub is_paired: bool,
    pub path: String,
    pub icon: String,
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
}

pub struct BluetoothService;

impl BluetoothService {
    pub fn spawn() -> (tokio::sync::watch::Receiver<BluetoothData>, mpsc::UnboundedSender<BluetoothCmd>) {
        let (data_tx, data_rx) = tokio::sync::watch::channel(BluetoothData::default());
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded::<BluetoothCmd>();

        tokio::spawn(async move {
            let connection = Connection::system().await.unwrap();
            let adapter_proxy = BluetoothAdapterProxy::new(&connection).await.unwrap();
            let obj_manager = ObjectManagerProxy::new(&connection).await.unwrap();

            let mut powered_changed = adapter_proxy.receive_powered_changed().await;
            let mut interfaces_added = obj_manager.receive_interfaces_added().await.unwrap();
            let mut interfaces_removed = obj_manager.receive_interfaces_removed().await.unwrap();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

            // Initial fetch
            Self::push_update(&adapter_proxy, &obj_manager, &connection, &data_tx).await;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::push_update(&adapter_proxy, &obj_manager, &connection, &data_tx).await;
                    }
                    Some(_) = powered_changed.next() => {
                        Self::push_update(&adapter_proxy, &obj_manager, &connection, &data_tx).await;
                    }
                    Some(_) = interfaces_added.next() => {
                        Self::push_update(&adapter_proxy, &obj_manager, &connection, &data_tx).await;
                    }
                    Some(_) = interfaces_removed.next() => {
                        Self::push_update(&adapter_proxy, &obj_manager, &connection, &data_tx).await;
                    }
                    Some(cmd) = cmd_rx.next() => {
                        match cmd {
                            BluetoothCmd::TogglePower(on) => { let _ = adapter_proxy.set_powered(on).await; }
                            BluetoothCmd::Scan => { let _ = adapter_proxy.start_discovery().await; }
                            BluetoothCmd::Connect(path_str) => {
                                if let Ok(path) = OwnedObjectPath::try_from(path_str) {
                                    if let Ok(dev_proxy) = BluetoothDeviceProxy::builder(&connection).path(path).unwrap().build().await {
                                        let _ = dev_proxy.connect().await;
                                    }
                                }
                            }
                            BluetoothCmd::Disconnect(path_str) => {
                                if let Ok(path) = OwnedObjectPath::try_from(path_str) {
                                    if let Ok(dev_proxy) = BluetoothDeviceProxy::builder(&connection).path(path).unwrap().build().await {
                                        let _ = dev_proxy.disconnect().await;
                                    }
                                }
                            }
                        }
                        Self::push_update(&adapter_proxy, &obj_manager, &connection, &data_tx).await;
                    }
                }
            }
        });

        (data_rx, cmd_tx)
    }

    async fn push_update(adapter: &BluetoothAdapterProxy<'_>, obj_manager: &ObjectManagerProxy<'_>, conn: &Connection, tx: &tokio::sync::watch::Sender<BluetoothData>) {
        let new_data = Self::get_current_data(adapter, obj_manager, conn).await;
        tx.send_if_modified(|current| {
            if *current != new_data {
                *current = new_data;
                true
            } else {
                false
            }
        });
    }

    async fn get_current_data(adapter: &BluetoothAdapterProxy<'_>, obj_manager: &ObjectManagerProxy<'_>, conn: &Connection) -> BluetoothData {
        let is_powered = adapter.powered().await.unwrap_or(false);
        let mut devices = Vec::new();

        if let Ok(objects) = obj_manager.get_managed_objects().await {
            for (path, interfaces) in objects {
                if interfaces.contains_key("org.bluez.Device1") {
                    if let Ok(dev_proxy) = BluetoothDeviceProxy::builder(conn).path(path.clone()).unwrap().build().await {
                        let name = match dev_proxy.name().await {
                            Ok(n) => n,
                            Err(_) => dev_proxy.alias().await.unwrap_or_else(|_| "Unknown Device".to_string()),
                        };
                        let is_connected = dev_proxy.connected().await.unwrap_or(false);
                        let is_paired = dev_proxy.paired().await.unwrap_or(false);
                        let icon = dev_proxy.icon().await.unwrap_or_else(|_| "bluetooth-symbolic".to_string());

                        devices.push(BluetoothDeviceData {
                            name,
                            is_connected,
                            is_paired,
                            path: path.to_string(),
                            icon,
                        });
                    }
                }
            }
        }

        devices.sort_by(|a, b| {
            if a.is_connected != b.is_connected {
                b.is_connected.cmp(&a.is_connected)
            } else if a.is_paired != b.is_paired {
                b.is_paired.cmp(&a.is_paired)
            } else {
                a.name.cmp(&b.name)
            }
        });

        BluetoothData { is_powered, devices }
    }
}
