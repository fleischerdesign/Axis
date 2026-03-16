use futures_util::StreamExt;
use zbus::{proxy, Connection, zvariant::OwnedObjectPath};
use async_channel::{Sender, Receiver, bounded};
use std::time::Duration;
use std::collections::HashMap;

#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn primary_connection_type(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property, name = "WirelessEnabled")]
    fn set_wireless_enabled(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn networking_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property, name = "NetworkingEnabled")]
    fn set_networking_enabled(&self, value: bool) -> zbus::Result<()>;

    fn get_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
    
    fn add_and_activate_connection(
        &self,
        connection: HashMap<&str, HashMap<&str, zbus::zvariant::Value<'_>>>,
        device: OwnedObjectPath,
        specific_object: OwnedObjectPath,
    ) -> zbus::Result<(OwnedObjectPath, OwnedObjectPath)>;

    fn activate_connection(
        &self,
        connection: OwnedObjectPath,
        device: OwnedObjectPath,
        specific_object: OwnedObjectPath,
    ) -> zbus::Result<OwnedObjectPath>;

    fn deactivate_connection(&self, connection: OwnedObjectPath) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Device {
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn active_connection(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
trait WirelessDevice {
    fn get_access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
    fn request_scan(&self, options: HashMap<String, zbus::zvariant::Value<'_>>) -> zbus::Result<()>;
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.AccessPoint",
    default_service = "org.freedesktop.NetworkManager"
)]
trait AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> zbus::Result<Vec<u8>>;
    #[zbus(property)]
    fn strength(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn flags(&self) -> zbus::Result<u32>;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessPointData {
    pub ssid: String,
    pub strength: u8,
    pub path: String,
    pub is_active: bool,
    pub needs_auth: bool,
}

pub enum NetworkCmd {
    ToggleWifi(bool),
    ScanWifi,
    ConnectToAp(String),
    ConnectToApWithPassword(String, String, String),
    DisconnectWifi,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NetworkData {
    pub is_wifi_connected: bool,
    pub is_ethernet_connected: bool,
    pub is_wifi_enabled: bool,
    pub active_strength: u8,
    pub access_points: Vec<AccessPointData>,
}

pub struct NetworkService;

impl NetworkService {
    pub fn spawn() -> (Receiver<NetworkData>, Sender<NetworkCmd>) {
        let (data_tx, data_rx) = bounded(100);
        let (cmd_tx, cmd_rx) = bounded(100);

        tokio::spawn(async move {
            let connection = Connection::system().await.unwrap();
            let proxy = NetworkManagerProxy::new(&connection).await.unwrap();

            let mut wifi_device_path = None;
            if let Ok(devices) = proxy.get_devices().await {
                for path in devices {
                    let dev_proxy = DeviceProxy::builder(&connection).path(path.clone()).unwrap().build().await.unwrap();
                    if let Ok(2) = dev_proxy.device_type().await {
                        wifi_device_path = Some(path);
                        break;
                    }
                }
            }

            let mut state_changed = proxy.receive_state_changed().await;
            let mut wifi_changed = proxy.receive_wireless_enabled_changed().await;
            let mut net_enabled_changed = proxy.receive_networking_enabled_changed().await;
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            let mut cmd_rx = Box::pin(cmd_rx);

            // Initial fetch
            Self::push_update(&proxy, &connection, wifi_device_path.as_ref(), &data_tx).await;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::push_update(&proxy, &connection, wifi_device_path.as_ref(), &data_tx).await;
                    }
                    Some(_) = state_changed.next() => {
                        Self::push_update(&proxy, &connection, wifi_device_path.as_ref(), &data_tx).await;
                    }
                    Some(_) = wifi_changed.next() => {
                        Self::push_update(&proxy, &connection, wifi_device_path.as_ref(), &data_tx).await;
                    }
                    Some(_) = net_enabled_changed.next() => {
                        Self::push_update(&proxy, &connection, wifi_device_path.as_ref(), &data_tx).await;
                    }
                    Some(cmd) = cmd_rx.next() => {
                        match cmd {
                            NetworkCmd::ToggleWifi(on) => { let _ = proxy.set_wireless_enabled(on).await; }
                            NetworkCmd::ScanWifi => {
                                if let Some(path) = &wifi_device_path {
                                    let wifi_proxy = WirelessDeviceProxy::builder(&connection).path(path).unwrap().build().await.unwrap();
                                    let _ = wifi_proxy.request_scan(HashMap::new()).await;
                                }
                            }
                            NetworkCmd::ConnectToAp(ap_path_str) => {
                                if let (Some(dev_path), Ok(ap_path)) = (&wifi_device_path, OwnedObjectPath::try_from(ap_path_str)) {
                                    let _ = proxy.activate_connection(OwnedObjectPath::try_from("/").unwrap(), dev_path.clone(), ap_path).await;
                                }
                            }
                            NetworkCmd::ConnectToApWithPassword(ap_path_str, ssid, password) => {
                                if let (Some(dev_path), Ok(ap_path)) = (&wifi_device_path, OwnedObjectPath::try_from(ap_path_str)) {
                                    let mut settings = HashMap::new();
                                    let mut conn_set = HashMap::new();
                                    conn_set.insert("type", zbus::zvariant::Value::from("802-11-wireless"));
                                    conn_set.insert("id", zbus::zvariant::Value::from(ssid.clone()));
                                    settings.insert("connection", conn_set);
                                    let mut wifi_set = HashMap::new();
                                    wifi_set.insert("ssid", zbus::zvariant::Value::from(ssid.as_bytes()));
                                    settings.insert("802-11-wireless", wifi_set);
                                    let mut security_set = HashMap::new();
                                    security_set.insert("key-mgmt", zbus::zvariant::Value::from("wpa-psk"));
                                    security_set.insert("psk", zbus::zvariant::Value::from(password));
                                    settings.insert("802-11-wireless-security", security_set);
                                    let _ = proxy.add_and_activate_connection(settings, dev_path.clone(), ap_path).await;
                                }
                            }
                            NetworkCmd::DisconnectWifi => {
                                if let Some(path) = &wifi_device_path {
                                    let dev_proxy = DeviceProxy::builder(&connection).path(path.clone()).unwrap().build().await.unwrap();
                                    if let Ok(active_conn_path) = dev_proxy.active_connection().await {
                                        if active_conn_path.to_string() != "/" {
                                            let _ = proxy.deactivate_connection(active_conn_path).await;
                                        }
                                    }
                                }
                            }
                        }
                        Self::push_update(&proxy, &connection, wifi_device_path.as_ref(), &data_tx).await;
                    }
                }
            }
        });

        (data_rx, cmd_tx)
    }

    async fn push_update(proxy: &NetworkManagerProxy<'_>, conn: &Connection, wifi_path: Option<&OwnedObjectPath>, tx: &Sender<NetworkData>) {
        let new_data = Self::get_current_data(proxy, conn, wifi_path).await;
        let _ = tx.send(new_data).await;
    }

    async fn get_current_data(proxy: &NetworkManagerProxy<'_>, conn: &Connection, wifi_path: Option<&OwnedObjectPath>) -> NetworkData {
        let state = proxy.state().await.unwrap_or(0);
        let wifi_on = proxy.wireless_enabled().await.unwrap_or(false);
        let primary_type = proxy.primary_connection_type().await.unwrap_or_default();
        
        let mut active_ap_path = "/".to_string();
        let mut active_ssid = String::new();
        let mut active_strength = 0;
        let mut aps = Vec::new();

        if let Some(path) = wifi_path {
            if let Ok(wifi_proxy) = WirelessDeviceProxy::builder(conn).path(path).unwrap().build().await {
                if let Ok(ap_path) = wifi_proxy.active_access_point().await {
                    active_ap_path = ap_path.to_string();
                    if let Ok(ap_proxy) = AccessPointProxy::builder(conn).path(ap_path).unwrap().build().await {
                        if let Ok(ssid_bytes) = ap_proxy.ssid().await {
                            active_ssid = String::from_utf8_lossy(&ssid_bytes).to_string();
                        }
                        if let Ok(s) = ap_proxy.strength().await {
                            active_strength = s;
                        }
                    }
                }

                if let Ok(ap_paths) = wifi_proxy.get_access_points().await {
                    for ap_path in ap_paths {
                        if let Ok(ap_proxy) = AccessPointProxy::builder(conn).path(ap_path.clone()).unwrap().build().await {
                            if let Ok(ssid_bytes) = ap_proxy.ssid().await {
                                let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();
                                if !ssid.is_empty() {
                                    let strength = ap_proxy.strength().await.unwrap_or(0);
                                    let flags = ap_proxy.flags().await.unwrap_or(0);
                                    let is_this_active = ap_path.to_string() == active_ap_path || (!active_ssid.is_empty() && ssid == active_ssid);

                                    aps.push(AccessPointData { 
                                        ssid, 
                                        strength, 
                                        path: ap_path.to_string(),
                                        is_active: is_this_active,
                                        needs_auth: flags != 0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        aps.sort_by(|a, b| b.strength.cmp(&a.strength));
        aps.dedup_by(|a, b| a.ssid == b.ssid);

        let is_connected = state >= 50 && state <= 70;

        NetworkData {
            is_wifi_connected: is_connected && (primary_type == "802-11-wireless" || active_ap_path != "/"),
            is_ethernet_connected: is_connected && primary_type == "802-3-ethernet",
            is_wifi_enabled: wifi_on,
            active_strength,
            access_points: aps,
        }
    }
}
