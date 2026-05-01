use axis_domain::models::bluetooth::{BluetoothDevice, BluetoothStatus, PairingType, PendingPairing};
use axis_domain::ports::bluetooth::{BluetoothStream, BluetoothError, BluetoothProvider};
use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::{watch, mpsc, oneshot};
use tokio_stream::wrappers::WatchStream;
use zbus::{proxy, zvariant::OwnedObjectPath, Connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::{info, warn};

#[proxy(
    interface = "org.bluez.Device1",
    default_service = "org.bluez"
)]
trait BluetoothDevice1 {
    #[zbus(property)]
    fn name(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn address(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn icon(&self) -> zbus::Result<String>;
    fn connect_dev(&self) -> zbus::Result<()>;
    fn disconnect_dev(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.bluez.Adapter1",
    default_service = "org.bluez"
)]
trait Adapter1 {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn discovering(&self) -> zbus::Result<bool>;
    fn start_discovery(&self) -> zbus::Result<()>;
    fn stop_discovery(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.DBus.ObjectManager",
    default_service = "org.bluez",
    default_path = "/"
)]
trait ObjectManager {
    fn get_managed_objects(
        &self,
    ) -> zbus::Result<
        HashMap<
            OwnedObjectPath,
            HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>,
        >,
    >;
    #[zbus(signal)]
    fn interfaces_added(
        &self,
        object: OwnedObjectPath,
        interfaces: HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>,
    ) -> zbus::Result<()>;
    #[zbus(signal)]
    fn interfaces_removed(
        &self,
        object: OwnedObjectPath,
        interfaces: Vec<String>,
    ) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.bluez.AgentManager1",
    default_service = "org.bluez",
    default_path = "/org/bluez"
)]
trait AgentManager1 {
    fn register_agent(&self, agent: &OwnedObjectPath, capability: &str) -> zbus::Result<()>;
    fn unregister_agent(&self, agent: &OwnedObjectPath) -> zbus::Result<()>;
    fn request_default_agent(&self, agent: &OwnedObjectPath) -> zbus::Result<()>;
}

enum PairResponse {
    Accept(Vec<u8>),
    Reject,
}

struct BluetoothAgent {
    pair_tx: mpsc::Sender<(PendingPairing, oneshot::Sender<PairResponse>)>,
}

#[zbus::interface(name = "org.bluez.Agent1")]
impl BluetoothAgent {
    async fn release(&self) {}

    async fn request_pin_code(
        &self,
        device: OwnedObjectPath,
    ) -> Result<String, zbus::fdo::Error> {
        let req = PendingPairing {
            device_path: device.to_string(),
            device_name: device.to_string(),
            passkey: None,
            pairing_type: PairingType::PinCode,
        };
        self.wait_for_response(req)
            .await
            .map(|s| String::from_utf8_lossy(&s).to_string())
    }

    async fn display_pin_code(&self, _device: OwnedObjectPath, _pincode: String) {}

    async fn request_passkey(
        &self,
        device: OwnedObjectPath,
    ) -> Result<u32, zbus::fdo::Error> {
        let req = PendingPairing {
            device_path: device.to_string(),
            device_name: device.to_string(),
            passkey: None,
            pairing_type: PairingType::Passkey,
        };
        let bytes = self.wait_for_response(req).await?;
        let s = String::from_utf8_lossy(&bytes);
        s.parse::<u32>().map_err(|_| zbus::fdo::Error::Failed("Invalid passkey".into()))
    }

    async fn display_passkey(&self, _device: OwnedObjectPath, _passkey: u32, _entered: u16) {}

    async fn request_confirmation(
        &self,
        device: OwnedObjectPath,
        passkey: u32,
    ) -> Result<(), zbus::fdo::Error> {
        let req = PendingPairing {
            device_path: device.to_string(),
            device_name: device.to_string(),
            passkey: Some(format!("{passkey:06}")),
            pairing_type: PairingType::Confirmation,
        };
        self.wait_for_response(req).await.map(|_| ())
    }

    async fn request_authorization(
        &self,
        device: OwnedObjectPath,
    ) -> Result<(), zbus::fdo::Error> {
        let req = PendingPairing {
            device_path: device.to_string(),
            device_name: device.to_string(),
            passkey: None,
            pairing_type: PairingType::Authorization,
        };
        self.wait_for_response(req).await.map(|_| ())
    }

    async fn authorize_service(
        &self,
        _device: OwnedObjectPath,
        _uuid: String,
    ) -> Result<(), zbus::fdo::Error> {
        Ok(())
    }

    async fn cancel(&self) {
        info!("[bluetooth] Agent cancelled");
    }
}

impl BluetoothAgent {
    fn reject() -> zbus::fdo::Error {
        zbus::fdo::Error::Failed("Rejected".into())
    }

    async fn wait_for_response(
        &self,
        req: PendingPairing,
    ) -> Result<Vec<u8>, zbus::fdo::Error> {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.pair_tx.send((req, resp_tx)).await.is_err() {
            return Err(Self::reject());
        }
        tokio::time::timeout(Duration::from_secs(30), resp_rx)
            .await
            .map_err(|_| Self::reject())?
            .map_err(|_| Self::reject())
            .and_then(|r| match r {
                PairResponse::Accept(v) => Ok(v),
                PairResponse::Reject => Err(Self::reject()),
            })
    }
}

pub struct BlueZProvider {
    status_tx: watch::Sender<BluetoothStatus>,
    connection: Connection,
    current_pair_response: Arc<Mutex<Option<oneshot::Sender<PairResponse>>>>,
}

impl BlueZProvider {
    pub async fn new() -> Result<Arc<Self>, BluetoothError> {
        let connection = Connection::system()
            .await
            .map_err(|e| BluetoothError::ProviderError(format!("System bus: {e}")))?;

        let powered = Self::get_adapter_powered(&connection).await;
        let initial_devices = Self::fetch_devices(&connection).await;
        info!(
            "[bluetooth] Initialized: powered={}, {} devices",
            powered,
            initial_devices.len()
        );

        let initial_status = BluetoothStatus {
            powered,
            is_scanning: false,
            devices: initial_devices,
            pending_pairing: None,
        };

        let (status_tx, _) = watch::channel(initial_status);
        let current_pair_response = Arc::new(Mutex::new(None::<oneshot::Sender<PairResponse>>));

        let provider = Arc::new(Self {
            status_tx,
            connection: connection.clone(),
            current_pair_response: current_pair_response.clone(),
        });

        let (pair_tx, mut pair_rx) = mpsc::channel::<(PendingPairing, oneshot::Sender<PairResponse>)>(4);

        let status_tx_p = provider.status_tx.clone();
        let cpr_p = current_pair_response.clone();
        tokio::spawn(async move {
            while let Some((pairing, resp_tx)) = pair_rx.recv().await {
                let mut current = status_tx_p.borrow().clone();
                current.pending_pairing = Some(pairing.clone());
                let _ = status_tx_p.send(current);

                *cpr_p.lock().unwrap() = Some(resp_tx);
            }
        });

        let conn_for_agent = connection.clone();
        let pair_tx_agent = pair_tx.clone();

        let agent = BluetoothAgent {
            pair_tx: pair_tx_agent,
        };

        let agent_path = OwnedObjectPath::try_from("/org/axis/bluetooth_agent")
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;

        match conn_for_agent.object_server().at("/org/axis/bluetooth_agent", agent).await {
            Ok(_) => {
        let agent_mgr = AgentManager1Proxy::builder(&conn_for_agent)
            .path("/org/bluez");
        if let Ok(builder) = agent_mgr {
            if let Ok(proxy) = builder.build().await {
                let _ = proxy.register_agent(&agent_path, "KeyboardDisplay").await;
                let _ = proxy.request_default_agent(&agent_path).await;
                info!("[bluetooth] BlueZ pairing agent registered");
            } else {
                warn!("[bluetooth] Failed to build AgentManager1 proxy");
            }
        } else {
            warn!("[bluetooth] Failed to create AgentManager1 proxy builder");
        }
            }
            Err(e) => {
                warn!("[bluetooth] Failed to register agent at D-Bus: {e}");
            }
        }

        let provider_clone = provider.clone();
        tokio::spawn(async move {
            let mut attempt = 0u32;
            loop {
                let om_proxy = match ObjectManagerProxy::new(&provider_clone.connection).await {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("[bluetooth] Failed to create ObjectManager proxy: {e}, retrying...");
                        attempt += 1;
                        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt.min(4)).min(30))).await;
                        continue;
                    }
                };

                let mut interfaces_added = match om_proxy.receive_interfaces_added().await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("[bluetooth] Failed to subscribe to interfaces_added: {e}, retrying...");
                        attempt += 1;
                        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt.min(4)).min(30))).await;
                        continue;
                    }
                };
                let mut interfaces_removed = match om_proxy.receive_interfaces_removed().await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("[bluetooth] Failed to subscribe to interfaces_removed: {e}, retrying...");
                        attempt += 1;
                        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt.min(4)).min(30))).await;
                        continue;
                    }
                };

                if attempt > 0 {
                    info!("[bluetooth] Reconnected to BlueZ");
                }
                attempt = 0;

                loop {
                    let alive = tokio::select! {
                        _ = interfaces_added.next() => true,
                        _ = interfaces_removed.next() => true,
                        else => false,
                    };

                    if !alive {
                        warn!("[bluetooth] BlueZ stream ended, reconnecting...");
                        break;
                    }

                    let devices = Self::fetch_devices(&provider_clone.connection).await;
                    let powered = Self::get_adapter_powered(&provider_clone.connection).await;
                    let current = provider_clone.status_tx.borrow().clone();
                    let _ = provider_clone.status_tx.send(BluetoothStatus {
                        powered,
                        is_scanning: current.is_scanning,
                        devices,
                        pending_pairing: current.pending_pairing,
                    });
                }

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });

        Ok(provider)
    }

    async fn get_adapter_powered(conn: &Connection) -> bool {
        let objects = match ObjectManagerProxy::new(conn).await {
            Ok(om) => om.get_managed_objects().await,
            Err(_) => return false,
        };

        let Ok(objects) = objects else { return false };

        for (path, interfaces) in objects {
            if interfaces.contains_key("org.bluez.Adapter1") {
                let path_str = path.to_string();
                let Ok(builder) = Adapter1Proxy::builder(conn).path(path) else {
                    log::warn!("[bluetooth] invalid adapter path: {path_str}");
                    continue;
                };
                if let Ok(proxy) = builder.build().await
                {
                    return proxy.powered().await.unwrap_or(false);
                }
            }
        }
        false
    }

    fn find_adapter_path(objects: &HashMap<OwnedObjectPath, HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>>) -> Option<OwnedObjectPath> {
        for (path, interfaces) in objects {
            if interfaces.contains_key("org.bluez.Adapter1") {
                return Some(path.clone());
            }
        }
        None
    }

    async fn fetch_devices(conn: &Connection) -> Vec<BluetoothDevice> {
        let Ok(om_proxy) = ObjectManagerProxy::new(conn).await else {
            return vec![];
        };

        let Ok(objects) = om_proxy.get_managed_objects().await else {
            return vec![];
        };

        let mut devices = Vec::new();
        for (path, interfaces) in objects {
            if !interfaces.contains_key("org.bluez.Device1") {
                continue;
            }

            let Ok(builder) = BluetoothDevice1Proxy::builder(conn).path(path.clone()) else {
                log::warn!("[bluetooth] invalid device path: {path}");
                continue;
            };
            let Ok(proxy) = builder.build().await else {
                continue;
            };

            let name = proxy.name().await.ok();
            let connected = proxy.connected().await.unwrap_or(false);
            let paired = proxy.paired().await.unwrap_or(false);
            let icon = proxy.icon().await.unwrap_or_else(|_| "bluetooth-symbolic".to_string());

            devices.push(BluetoothDevice {
                id: path.to_string(),
                name,
                connected,
                paired,
                icon,
            });
        }

        devices.sort_by(|a, b| {
            b.connected
                .cmp(&a.connected)
                .then_with(|| b.paired.cmp(&a.paired))
        });

        devices
    }
}

#[async_trait]
impl BluetoothProvider for BlueZProvider {
    async fn get_status(&self) -> Result<BluetoothStatus, BluetoothError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<BluetoothStream, BluetoothError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn connect(&self, id: &str) -> Result<(), BluetoothError> {
        let path = OwnedObjectPath::try_from(id.to_string())
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;
        let proxy = BluetoothDevice1Proxy::builder(&self.connection)
            .path(path)
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| BluetoothError::ConnectionFailed(format!("Proxy: {e}")))?;
        proxy
            .connect_dev()
            .await
            .map_err(|e| BluetoothError::ConnectionFailed(format!("Connect: {e}")))?;
        info!("[bluetooth] Connected: {id}");
        let devices = Self::fetch_devices(&self.connection).await;
        let prev = self.status_tx.borrow().clone();
        let _ = self.status_tx.send(BluetoothStatus { devices, ..prev });
        Ok(())
    }

    async fn disconnect(&self, id: &str) -> Result<(), BluetoothError> {
        let path = OwnedObjectPath::try_from(id.to_string())
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;
        let proxy = BluetoothDevice1Proxy::builder(&self.connection)
            .path(path)
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| BluetoothError::ProviderError(format!("Proxy: {e}")))?;
        proxy
            .disconnect_dev()
            .await
            .map_err(|e| BluetoothError::ProviderError(format!("Disconnect: {e}")))?;
        info!("[bluetooth] Disconnected: {id}");
        let devices = Self::fetch_devices(&self.connection).await;
        let prev = self.status_tx.borrow().clone();
        let _ = self.status_tx.send(BluetoothStatus { devices, ..prev });
        Ok(())
    }

    async fn set_powered(&self, powered: bool) -> Result<(), BluetoothError> {
        let om_proxy = ObjectManagerProxy::new(&self.connection)
            .await
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;
        let objects = om_proxy
            .get_managed_objects()
            .await
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;

        if let Some(adapter_path) = Self::find_adapter_path(&objects) {
            let proxy = Adapter1Proxy::builder(&self.connection)
                .path(adapter_path)
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?
                .build()
                .await
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;

            proxy
                .set_powered(powered)
                .await
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;
            info!("[bluetooth] Set powered: {powered}");

            let prev = self.status_tx.borrow().clone();
            let _ = self.status_tx.send(BluetoothStatus { powered, ..prev });
        }
        Ok(())
    }

    async fn start_scan(&self) -> Result<(), BluetoothError> {
        let om_proxy = ObjectManagerProxy::new(&self.connection)
            .await
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;
        let objects = om_proxy
            .get_managed_objects()
            .await
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;

        if let Some(adapter_path) = Self::find_adapter_path(&objects) {
            let proxy = Adapter1Proxy::builder(&self.connection)
                .path(adapter_path)
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?
                .build()
                .await
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;

            proxy
                .start_discovery()
                .await
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;
            info!("[bluetooth] Started scan");

            let prev = self.status_tx.borrow().clone();
            let _ = self.status_tx.send(BluetoothStatus { is_scanning: true, ..prev });
        }
        Ok(())
    }

    async fn stop_scan(&self) -> Result<(), BluetoothError> {
        let om_proxy = ObjectManagerProxy::new(&self.connection)
            .await
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;
        let objects = om_proxy
            .get_managed_objects()
            .await
            .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;

        if let Some(adapter_path) = Self::find_adapter_path(&objects) {
            let proxy = Adapter1Proxy::builder(&self.connection)
                .path(adapter_path)
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?
                .build()
                .await
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;

            proxy
                .stop_discovery()
                .await
                .map_err(|e| BluetoothError::ProviderError(e.to_string()))?;
            info!("[bluetooth] Stopped scan");

            let prev = self.status_tx.borrow().clone();
            let _ = self.status_tx.send(BluetoothStatus { is_scanning: false, ..prev });
        }
        Ok(())
    }

    async fn pair_accept(&self) -> Result<(), BluetoothError> {
        let response = self.current_pair_response.lock().unwrap().take();
        if let Some(tx) = response {
            let _ = tx.send(PairResponse::Accept(Vec::new()));
            self.status_tx.send_modify(|s| s.pending_pairing = None);
        }
        Ok(())
    }

    async fn pair_reject(&self) -> Result<(), BluetoothError> {
        let response = self.current_pair_response.lock().unwrap().take();
        if let Some(tx) = response {
            let _ = tx.send(PairResponse::Reject);
            self.status_tx.send_modify(|s| s.pending_pairing = None);
        }
        Ok(())
    }
}
