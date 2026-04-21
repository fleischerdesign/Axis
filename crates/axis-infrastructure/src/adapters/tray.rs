use axis_domain::models::tray::{IconPixmap, TrayItem, TrayItemStatus, TrayStatus};
use axis_domain::ports::tray::{TrayError, TrayProvider, TrayStream};
use async_trait::async_trait;
use futures_util::StreamExt;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::WatchStream;
use zbus::names::BusName;
use zbus::zvariant::ObjectPath;
use zbus::{proxy, Connection};

#[proxy(
    interface = "org.kde.StatusNotifierItem",
    default_service = "org.kde.StatusNotifierItem",
    default_path = "/StatusNotifierItem"
)]
trait StatusNotifierItem {
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;
    #[zbus(property)]
    fn overlay_icon_name(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn attention_icon_name(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn scroll(&self, delta: i32, orientation: &str) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_attention_icon(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_title(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn new_status(&self, status: &str) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.DBus",
    default_service = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus"
)]
trait DBus {
    fn list_names(&self) -> zbus::Result<Vec<String>>;
    #[zbus(signal)]
    fn name_owner_changed(
        &self,
        name: String,
        old_owner: String,
        new_owner: String,
    ) -> zbus::Result<()>;
}

struct StatusNotifierWatcherIface {
    reg_tx: mpsc::Sender<String>,
}

#[zbus::interface(name = "org.freedesktop.StatusNotifierWatcher")]
impl StatusNotifierWatcherIface {
    fn register_status_notifier_item(&self, service: &str) {
        let _ = self.reg_tx.try_send(service.to_string());
    }

    fn register_status_notifier_host(&self, _service: &str) {}

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        vec![]
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

struct StatusNotifierWatcherKdeIface {
    reg_tx: mpsc::Sender<String>,
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcherKdeIface {
    fn register_status_notifier_item(&self, service: &str) {
        let _ = self.reg_tx.try_send(service.to_string());
    }

    fn register_status_notifier_host(&self, _service: &str) {}

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        vec![]
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

enum ItemEvent {
    Updated { bus_name: String, item: TrayItem },
    Disconnected(String),
}

pub struct StatusNotifierAdapter {
    status_tx: watch::Sender<TrayStatus>,
    connection: Connection,
}

impl StatusNotifierAdapter {
    pub async fn new() -> Result<Arc<Self>, TrayError> {
        let (status_tx, _) = watch::channel(TrayStatus::default());
        let (reg_tx, reg_rx) = mpsc::channel::<String>(64);
        let (event_tx, event_rx) = mpsc::unbounded_channel::<ItemEvent>();

        let conn = Connection::session()
            .await
            .map_err(|e| TrayError::ProviderError(format!("Session bus: {e}")))?;

        let watcher = StatusNotifierWatcherIface {
            reg_tx: reg_tx.clone(),
        };
        let watcher_kde = StatusNotifierWatcherKdeIface { reg_tx };

        conn.object_server()
            .at("/StatusNotifierWatcher", watcher)
            .await
            .map_err(|e| TrayError::ProviderError(format!("Serve watcher: {e}")))?;
        conn.object_server()
            .at("/StatusNotifierWatcher", watcher_kde)
            .await
            .map_err(|e| TrayError::ProviderError(format!("Serve KDE watcher: {e}")))?;

        conn.request_name("org.freedesktop.StatusNotifierWatcher")
            .await
            .map_err(|e| TrayError::ProviderError(format!("Request watcher name: {e}")))?;
        conn.request_name("org.kde.StatusNotifierWatcher")
            .await
            .map_err(|e| TrayError::ProviderError(format!("Request KDE watcher name: {e}")))?;

        let host_name = format!("org.freedesktop.StatusNotifierHost-{}", std::process::id());
        conn.request_name(host_name.as_str())
            .await
            .map_err(|e| TrayError::ProviderError(format!("Request host name: {e}")))?;

        info!("[tray] StatusNotifierWatcher registered on D-Bus");

        let dbus_proxy = DBusProxy::new(&conn)
            .await
            .map_err(|e| TrayError::ProviderError(format!("DBus proxy: {e}")))?;

        let existing_names = dbus_proxy.list_names().await.unwrap_or_default();

        let mut items: Vec<TrayItem> = Vec::new();

        for name in &existing_names {
            if name.starts_with("org.freedesktop.StatusNotifierItem-") {
                if let Some(item) =
                    Self::add_item(&conn, name, "/StatusNotifierItem", &event_tx).await
                {
                    info!("[tray] Found existing item: {}", name);
                    items.push(item);
                }
            }
        }

        let _ = status_tx.send(TrayStatus { items });

        let provider = Arc::new(Self {
            status_tx,
            connection: conn.clone(),
        });

        let provider_clone = provider.clone();
        tokio::spawn(async move {
            Self::run_event_loop(
                &provider_clone.connection,
                reg_rx,
                event_rx,
                &provider_clone.status_tx,
            )
            .await;
        });

        Ok(provider)
    }

    fn parse_bus_name(dest: &str) -> Result<BusName<'static>, TrayError> {
        dest.to_string()
            .try_into()
            .map_err(|e| TrayError::ProviderError(format!("Invalid bus name '{dest}': {e}")))
    }

    fn parse_object_path(path: &str) -> Result<ObjectPath<'static>, TrayError> {
        path.to_string()
            .try_into()
            .map_err(|e| TrayError::ProviderError(format!("Invalid object path '{path}': {e}")))
    }

    async fn add_item(
        conn: &Connection,
        destination: &str,
        path: &str,
        event_tx: &mpsc::UnboundedSender<ItemEvent>,
    ) -> Option<TrayItem> {
        let dest = match Self::parse_bus_name(destination) {
            Ok(d) => d,
            Err(e) => {
                warn!("[tray] {e}");
                return None;
            }
        };
        let obj_path = match Self::parse_object_path(path) {
            Ok(p) => p,
            Err(e) => {
                warn!("[tray] {e}");
                return None;
            }
        };

        let proxy = match StatusNotifierItemProxy::builder(conn)
            .destination(dest)
            .expect("invalid destination")
            .path(obj_path)
            .expect("invalid path")
            .build()
            .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!("[tray] Failed to create proxy for {destination}: {e}");
                return None;
            }
        };

        let item = Self::fetch_item_properties(&proxy, destination).await;

        let tx = event_tx.clone();
        let bus_name = destination.to_string();
        tokio::spawn(async move {
            let mut new_icon = match proxy.receive_new_icon().await {
                Ok(s) => s,
                Err(_) => return,
            };
            let mut new_attention_icon = match proxy.receive_new_attention_icon().await {
                Ok(s) => s,
                Err(_) => return,
            };
            let mut new_title = match proxy.receive_new_title().await {
                Ok(s) => s,
                Err(_) => return,
            };
            let mut new_status = match proxy.receive_new_status().await {
                Ok(s) => s,
                Err(_) => return,
            };

            loop {
                let alive = tokio::select! {
                    _ = new_icon.next() => true,
                    _ = new_attention_icon.next() => true,
                    _ = new_title.next() => true,
                    _ = new_status.next() => true,
                    else => false,
                };

                if !alive {
                    let _ = tx.send(ItemEvent::Disconnected(bus_name.clone()));
                    break;
                }

                let updated = Self::fetch_item_properties(&proxy, &bus_name).await;
                let _ = tx.send(ItemEvent::Updated {
                    bus_name: bus_name.clone(),
                    item: updated,
                });
            }
        });

        Some(item)
    }

    async fn fetch_item_properties(proxy: &StatusNotifierItemProxy<'_>, bus_name: &str) -> TrayItem {
        let id = proxy.id().await.unwrap_or_default();
        let title = proxy.title().await.unwrap_or_default();
        let icon_name = proxy.icon_name().await.unwrap_or_default();
        let attention_icon_name = proxy.attention_icon_name().await.unwrap_or_default();
        let overlay_icon_name = proxy.overlay_icon_name().await.unwrap_or_default();
        let status_str = proxy.status().await.unwrap_or_else(|_| "Active".to_string());
        let icon_pixmap = proxy
            .icon_pixmap()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(w, h, d)| IconPixmap {
                width: w,
                height: h,
                data: d,
            })
            .collect();
        let attention_icon_pixmap = proxy
            .attention_icon_pixmap()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(w, h, d)| IconPixmap {
                width: w,
                height: h,
                data: d,
            })
            .collect();

        TrayItem {
            bus_name: bus_name.to_string(),
            id,
            title,
            icon_name,
            attention_icon_name,
            overlay_icon_name,
            icon_pixmap,
            attention_icon_pixmap,
            status: Self::parse_status(&status_str),
        }
    }

    fn parse_status(s: &str) -> TrayItemStatus {
        match s {
            "Passive" => TrayItemStatus::Passive,
            "NeedsAttention" => TrayItemStatus::NeedsAttention,
            _ => TrayItemStatus::Active,
        }
    }

    fn parse_registration_item(item_id: &str) -> (String, String) {
        if item_id.contains('/') {
            if let Some(slash) = item_id.find('/') {
                return (
                    item_id[..slash].to_string(),
                    item_id[slash..].to_string(),
                );
            }
        }
        (item_id.to_string(), "/StatusNotifierItem".to_string())
    }

    async fn run_event_loop(
        conn: &Connection,
        mut reg_rx: mpsc::Receiver<String>,
        mut event_rx: mpsc::UnboundedReceiver<ItemEvent>,
        status_tx: &watch::Sender<TrayStatus>,
    ) {
        let dbus_proxy = match DBusProxy::new(conn).await {
            Ok(p) => p,
            Err(e) => {
                warn!("[tray] Failed to create DBus proxy: {e}");
                return;
            }
        };

        let mut name_changed = match dbus_proxy.receive_name_owner_changed().await {
            Ok(s) => s,
            Err(e) => {
                warn!("[tray] Failed to subscribe to NameOwnerChanged: {e}");
                return;
            }
        };

        let event_tx_reg = {
            let (tx, mut rx) = mpsc::unbounded_channel::<ItemEvent>();
            tokio::spawn(async move {
                while rx.recv().await.is_some() {}
            });
            tx
        };

        loop {
            tokio::select! {
                Some(reg_item) = reg_rx.recv() => {
                    let (destination, path) = Self::parse_registration_item(&reg_item);

                    let is_dup = status_tx.borrow().items.iter().any(|i| i.bus_name == destination);
                    if is_dup {
                        continue;
                    }

                    if let Some(item) = Self::add_item(conn, &destination, &path, &event_tx_reg).await {
                        info!("[tray] Registered item: {destination}");
                        let mut current = status_tx.borrow().clone();
                        current.items.push(item);
                        let _ = status_tx.send(current);
                    }
                }

                Some(change) = name_changed.next() => {
                    let args = match change.args() {
                        Ok(a) => a,
                        Err(_) => continue,
                    };
                    let name = args.name();
                    let new_owner = args.new_owner();

                    if name.starts_with("org.freedesktop.StatusNotifierItem-")
                        && new_owner.is_empty()
                    {
                        let mut current = status_tx.borrow().clone();
                        let before = current.items.len();
                        current.items.retain(|i| i.bus_name.as_str() != name);
                        if current.items.len() < before {
                            info!("[tray] Item disconnected: {name}");
                            let _ = status_tx.send(current);
                        }
                    }
                }

                Some(event) = event_rx.recv() => {
                    match event {
                        ItemEvent::Updated { bus_name, item } => {
                            let mut current = status_tx.borrow().clone();
                            if let Some(existing) = current.items.iter_mut().find(|i| i.bus_name == bus_name) {
                                *existing = item;
                            }
                            let _ = status_tx.send(current);
                        }
                        ItemEvent::Disconnected(bus_name) => {
                            let mut current = status_tx.borrow().clone();
                            let before = current.items.len();
                            current.items.retain(|i| i.bus_name != bus_name);
                            if current.items.len() < before {
                                info!("[tray] Item disconnected: {bus_name}");
                                let _ = status_tx.send(current);
                            }
                        }
                    }
                }

                else => break,
            }
        }
    }
}

#[async_trait]
impl TrayProvider for StatusNotifierAdapter {
    async fn get_status(&self) -> Result<TrayStatus, TrayError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<TrayStream, TrayError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn activate(&self, bus_name: &str, x: i32, y: i32) -> Result<(), TrayError> {
        let dest = Self::parse_bus_name(bus_name)?;
        let proxy = StatusNotifierItemProxy::builder(&self.connection)
            .destination(dest)
            .map_err(|e| TrayError::ProviderError(e.to_string()))?
            .path("/StatusNotifierItem")
            .map_err(|e| TrayError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| TrayError::ProviderError(format!("Proxy: {e}")))?;
        proxy
            .activate(x, y)
            .await
            .map_err(|e| TrayError::ProviderError(format!("Activate: {e}")))?;
        Ok(())
    }

    async fn context_menu(&self, bus_name: &str, x: i32, y: i32) -> Result<(), TrayError> {
        let dest = Self::parse_bus_name(bus_name)?;
        let proxy = StatusNotifierItemProxy::builder(&self.connection)
            .destination(dest)
            .map_err(|e| TrayError::ProviderError(e.to_string()))?
            .path("/StatusNotifierItem")
            .map_err(|e| TrayError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| TrayError::ProviderError(format!("Proxy: {e}")))?;
        proxy
            .context_menu(x, y)
            .await
            .map_err(|e| TrayError::ProviderError(format!("ContextMenu: {e}")))?;
        Ok(())
    }

    async fn secondary_activate(&self, bus_name: &str, x: i32, y: i32) -> Result<(), TrayError> {
        let dest = Self::parse_bus_name(bus_name)?;
        let proxy = StatusNotifierItemProxy::builder(&self.connection)
            .destination(dest)
            .map_err(|e| TrayError::ProviderError(e.to_string()))?
            .path("/StatusNotifierItem")
            .map_err(|e| TrayError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| TrayError::ProviderError(format!("Proxy: {e}")))?;
        proxy
            .secondary_activate(x, y)
            .await
            .map_err(|e| TrayError::ProviderError(format!("SecondaryActivate: {e}")))?;
        Ok(())
    }

    async fn scroll(&self, bus_name: &str, delta: i32, orientation: &str) -> Result<(), TrayError> {
        let dest = Self::parse_bus_name(bus_name)?;
        let proxy = StatusNotifierItemProxy::builder(&self.connection)
            .destination(dest)
            .map_err(|e| TrayError::ProviderError(e.to_string()))?
            .path("/StatusNotifierItem")
            .map_err(|e| TrayError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| TrayError::ProviderError(format!("Proxy: {e}")))?;
        proxy
            .scroll(delta, orientation)
            .await
            .map_err(|e| TrayError::ProviderError(format!("Scroll: {e}")))?;
        Ok(())
    }
}
