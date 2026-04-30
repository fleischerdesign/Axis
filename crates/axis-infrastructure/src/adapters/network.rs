use axis_domain::models::network::{AccessPoint, NetworkStatus};
use axis_domain::ports::network::{NetworkError, NetworkProvider, NetworkStream};
use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use zbus::{proxy, zvariant::OwnedObjectPath, Connection};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use log::{info, warn};

const MAX_ACCESS_POINTS: usize = 15;

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
    fn get_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
    fn activate_connection(
        &self,
        connection: OwnedObjectPath,
        device: OwnedObjectPath,
        specific_object: OwnedObjectPath,
    ) -> zbus::Result<OwnedObjectPath>;
    fn deactivate_connection(&self, connection: OwnedObjectPath) -> zbus::Result<()>;
    fn add_and_activate_connection(
        &self,
        connection: HashMap<&str, HashMap<&str, zbus::zvariant::Value<'_>>>,
        device: OwnedObjectPath,
        specific_object: OwnedObjectPath,
    ) -> zbus::Result<(OwnedObjectPath, OwnedObjectPath)>;
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
    fn request_scan(
        &self,
        options: HashMap<String, zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    fn access_point_added(
        &self,
        access_point: zbus::zvariant::OwnedObjectPath,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    fn access_point_removed(
        &self,
        access_point: zbus::zvariant::OwnedObjectPath,
    ) -> zbus::Result<()>;
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;
    #[zbus(property)]
    fn last_scan(&self) -> zbus::Result<i64>;
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

pub struct NetworkManagerProvider {
    status_tx: watch::Sender<NetworkStatus>,
    connection: Connection,
    nm_proxy: NetworkManagerProxy<'static>,
    wifi_device_path: Option<OwnedObjectPath>,
}

impl NetworkManagerProvider {
    pub async fn new() -> Result<Arc<Self>, NetworkError> {
        let connection = Connection::system()
            .await
            .map_err(|e| NetworkError::ProviderError(format!("System bus: {e}")))?;

        let nm_proxy = NetworkManagerProxy::new(&connection)
            .await
            .map_err(|e| NetworkError::ProviderError(format!("NM proxy: {e}")))?;

        let wifi_device_path = Self::find_wifi_device(&nm_proxy, &connection).await;

        let initial_status =
            Self::fetch_data(&nm_proxy, &connection, wifi_device_path.as_ref()).await;
        info!(
            "[network] Initialized: wifi={}, ethernet={}, aps={}",
            initial_status.is_wifi_connected,
            initial_status.is_ethernet_connected,
            initial_status.access_points.len()
        );

        let (status_tx, _) = watch::channel(initial_status);

        let provider = Arc::new(Self {
            status_tx,
            connection: connection.clone(),
            nm_proxy,
            wifi_device_path: wifi_device_path.clone(),
        });

        let provider_clone = provider.clone();
        let conn = connection.clone();

        tokio::spawn(async move {
            let mut attempt = 0u32;
            loop {
                let nm_proxy = match NetworkManagerProxy::new(&conn).await {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("[network] Failed to create NM proxy: {e}, retrying...");
                        attempt += 1;
                        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt.min(4)).min(30))).await;
                        continue;
                    }
                };

                if attempt > 0 {
                    info!("[network] Reconnected to NetworkManager");
                }
                attempt = 0;

                let wifi_device_path = Self::find_wifi_device(&nm_proxy, &conn).await;

                let mut state_changed = nm_proxy.receive_state_changed().await;
                let mut wifi_changed = nm_proxy.receive_wireless_enabled_changed().await;

                let mut ap_added_stream = None;
                let mut ap_removed_stream = None;
                let mut last_scan_stream = None;

                if let Some(ref wp) = wifi_device_path {
                    let Ok(builder) = WirelessDeviceProxy::builder(&conn).path(wp.clone()) else {
                        log::warn!("[network] invalid wifi path: {wp}");
                        continue;
                    };
                    if let Ok(wifi_proxy) = builder.build().await
                    {
                        ap_added_stream = wifi_proxy.receive_access_point_added().await.ok();
                        ap_removed_stream = wifi_proxy.receive_access_point_removed().await.ok();
                        last_scan_stream = Some(wifi_proxy.receive_last_scan_changed().await);
                    }
                }

                loop {
                    let alive = tokio::select! {
                        _ = state_changed.next() => true,
                        _ = wifi_changed.next() => true,
                        Some(_) = async { ap_added_stream.as_mut()?.next().await }, if ap_added_stream.is_some() => true,
                        Some(_) = async { ap_removed_stream.as_mut()?.next().await }, if ap_removed_stream.is_some() => true,
                        Some(_) = async { last_scan_stream.as_mut()?.next().await }, if last_scan_stream.is_some() => true,
                        else => false,
                    };

                    if !alive {
                        warn!("[network] NM stream ended, reconnecting...");
                        break;
                    }

                    let status = Self::fetch_data(
                        &nm_proxy,
                        &conn,
                        wifi_device_path.as_ref(),
                    ).await;
                    let _ = provider_clone.status_tx.send(status);
                }

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        Ok(provider)
    }

    async fn find_wifi_device(
        nm_proxy: &NetworkManagerProxy<'_>,
        conn: &Connection,
    ) -> Option<OwnedObjectPath> {
        let devices = nm_proxy.get_devices().await.ok()?;
        for path in devices {
            let Ok(builder) = DeviceProxy::builder(conn).path(path.clone()) else {
                log::warn!("[network] invalid device path: {path}");
                continue;
            };
            if let Ok(dev_proxy) = builder.build().await
            {
                if let Ok(2) = dev_proxy.device_type().await {
                    info!("[network] WiFi device found: {path}");
                    return Some(path);
                }
            }
        }
        None
    }

    async fn fetch_data(
        nm_proxy: &NetworkManagerProxy<'_>,
        conn: &Connection,
        wifi_path: Option<&OwnedObjectPath>,
    ) -> NetworkStatus {
        let state = nm_proxy.state().await.unwrap_or(0);
        let wifi_on = nm_proxy.wireless_enabled().await.unwrap_or(false);
        let primary_type = nm_proxy
            .primary_connection_type()
            .await
            .unwrap_or_default();

        let mut active_ap_path = "/".to_string();
        let mut active_strength = 0u8;
        let mut aps = Vec::new();

        if let Some(path) = wifi_path {
            let Ok(wifi_builder) = WirelessDeviceProxy::builder(conn).path(path) else {
                log::warn!("[network] invalid wifi path: {path}");
                return NetworkStatus::default();
            };
            if let Ok(wifi_proxy) = wifi_builder.build().await
            {
                if let Ok(ap_path) = wifi_proxy.active_access_point().await {
                        active_ap_path = ap_path.to_string();
                        let ap_path_str = active_ap_path.clone();
                        match AccessPointProxy::builder(conn).path(ap_path) {
                            Ok(builder) => {
                                if let Ok(ap_proxy) = builder.build().await {
                                    active_strength = ap_proxy.strength().await.unwrap_or(0);
                                }
                            }
                            Err(e) => log::warn!("[network] invalid ap path {ap_path_str}: {e}"),
                        }
                }

                if let Ok(ap_paths) = wifi_proxy.get_access_points().await {
                    for ap_path in ap_paths.into_iter().take(MAX_ACCESS_POINTS) {
                        let Ok(ap_builder) = AccessPointProxy::builder(conn).path(ap_path.clone()) else {
                            log::warn!("[network] invalid ap path: {ap_path}");
                            continue;
                        };
                        if let Ok(ap_proxy) = ap_builder.build().await
                        {
                            if let Ok(ssid_bytes) = ap_proxy.ssid().await {
                                let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();
                                if !ssid.is_empty() {
                                    let ap_path_str = ap_path.to_string();
                                    aps.push(AccessPoint {
                                        id: ap_path_str.clone(),
                                        ssid,
                                        strength: ap_proxy.strength().await.unwrap_or(0),
                                        is_active: ap_path_str == active_ap_path,
                                        needs_auth: ap_proxy.flags().await.unwrap_or(0) != 0,
                                    });
                                }
                            }
                        }
                    }
                    aps.sort_by(|a, b| {
                        b.is_active
                            .cmp(&a.is_active)
                            .then_with(|| b.strength.cmp(&a.strength))
                    });
                    let mut seen = HashSet::new();
                    aps.retain(|ap| seen.insert(ap.ssid.clone()));
                }
            }
        }

        let is_connected = state >= 50 && state <= 70;
        NetworkStatus {
            is_wifi_connected: is_connected
                && (primary_type == "802-11-wireless" || active_ap_path != "/"),
            is_ethernet_connected: is_connected && primary_type == "802-3-ethernet",
            is_wifi_enabled: wifi_on,
            active_strength,
            is_scanning: false,
            access_points: aps,
        }
    }
}

#[async_trait]
impl NetworkProvider for NetworkManagerProvider {
    async fn get_status(&self) -> Result<NetworkStatus, NetworkError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<NetworkStream, NetworkError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn set_wifi_enabled(&self, enabled: bool) -> Result<(), NetworkError> {
        self.nm_proxy
            .set_wireless_enabled(enabled)
            .await
            .map_err(|e| NetworkError::ProviderError(format!("Toggle WiFi: {e}")))?;
        info!(
            "[network] WiFi {}",
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    }

    async fn scan_wifi(&self) -> Result<(), NetworkError> {
        if let Some(path) = &self.wifi_device_path {
            let wifi_proxy = WirelessDeviceProxy::builder(&self.connection)
                .path(path.clone())
                .map_err(|e| NetworkError::ProviderError(e.to_string()))?
                .build()
                .await
                .map_err(|e| NetworkError::ProviderError(format!("WiFi proxy: {e}")))?;
            wifi_proxy
                .request_scan(HashMap::new())
                .await
                .map_err(|e| NetworkError::ProviderError(format!("Scan: {e}")))?;
            info!("[network] WiFi scan initiated");
        }
        Ok(())
    }

    async fn connect_to_ap(
        &self,
        id: &str,
        password: Option<&str>,
    ) -> Result<(), NetworkError> {
        let dev_path = self
            .wifi_device_path
            .clone()
            .ok_or_else(|| NetworkError::ProviderError("No WiFi device".into()))?;
        let ap_path = OwnedObjectPath::try_from(id.to_string())
            .map_err(|e| NetworkError::ProviderError(e.to_string()))?;

        if let Some(pw) = password {
            let mut settings = HashMap::new();
            let mut conn_set = HashMap::new();
            conn_set.insert("type", zbus::zvariant::Value::from("802-11-wireless"));
            conn_set.insert("id", zbus::zvariant::Value::from(id));
            settings.insert("connection", conn_set);

            let mut wifi_set = HashMap::new();
            wifi_set.insert("ssid", zbus::zvariant::Value::from(id.as_bytes()));
            settings.insert("802-11-wireless", wifi_set);

            let mut security_set = HashMap::new();
            security_set.insert("key-mgmt", zbus::zvariant::Value::from("wpa-psk"));
            security_set.insert("psk", zbus::zvariant::Value::from(pw));
            settings.insert("802-11-wireless-security", security_set);

            self.nm_proxy
                .add_and_activate_connection(settings, dev_path, ap_path)
                .await
                .map_err(|e| NetworkError::ProviderError(format!("Connect: {e}")))?;
        } else {
            let root_path = OwnedObjectPath::try_from("/").unwrap();
            self.nm_proxy
                .activate_connection(root_path, dev_path, ap_path)
                .await
                .map_err(|e| NetworkError::ProviderError(format!("Connect: {e}")))?;
        }
        info!("[network] Connecting to AP: {id}");
        Ok(())
    }

    async fn disconnect_wifi(&self) -> Result<(), NetworkError> {
        if let Some(path) = &self.wifi_device_path {
            let dev_proxy = DeviceProxy::builder(&self.connection)
                .path(path.clone())
                .map_err(|e| NetworkError::ProviderError(e.to_string()))?
                .build()
                .await
                .map_err(|e| NetworkError::ProviderError(format!("Device proxy: {e}")))?;
            if let Ok(active_conn_path) = dev_proxy.active_connection().await {
                if active_conn_path.to_string() != "/" {
                    self.nm_proxy
                        .deactivate_connection(active_conn_path)
                        .await
                        .map_err(|e| NetworkError::ProviderError(format!("Disconnect: {e}")))?;
                    info!("[network] Disconnected from WiFi");
                }
            }
        }
        Ok(())
    }
}
