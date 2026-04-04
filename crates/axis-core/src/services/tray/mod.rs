pub mod item_proxy;

use super::Service;
use crate::store::ServiceStore;
use async_channel::{bounded, Sender};
use futures_util::StreamExt;
use item_proxy::StatusNotifierItemProxy;
use log::{error, info, warn};
use std::collections::HashMap;
use zbus::interface;
use zbus::fdo::DBusProxy;
use zbus::Connection;
use zbus::message::Header;
use zbus::names::WellKnownName;

// --- Data Types ---

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayItem {
    pub bus_name: String,
    pub id: String,
    pub title: String,
    pub icon_name: String,
    pub attention_icon_name: String,
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrayData {
    pub items: Vec<TrayItem>,
}

#[derive(Debug)]
pub enum TrayCmd {
    Activate(String),
    ContextMenu(String),
    SecondaryActivate(String),
    Scroll(String, i32, String),
}

// Internal messages from per-item tasks
enum ItemEvent {
    Updated { bus_name: String, title: String, icon_name: String, attention_icon_name: String, status: String },
    Disconnected(String),
}

// --- D-Bus Watcher Interface ---
//
// Both the freedesktop and KDE interface names are served.
// Apps like Steam and Discord use the KDE name, others use freedesktop.
// The macro generates two distinct structs with identical implementations.

macro_rules! define_status_notifier_watcher {
    ($ty:ident, $name:literal) => {
        struct $ty {
            reg_tx: Sender<String>,
        }

        #[interface(name = $name)]
        impl $ty {
            fn register_status_notifier_item(&self, service: String, #[zbus(header)] header: Header<'_>) {
                let item = if service.starts_with('/') {
                    if let Some(sender) = header.sender() {
                        format!("{sender}{service}")
                    } else {
                        service
                    }
                } else {
                    service
                };
                info!("[tray] RegisterStatusNotifierItem: {item}");
                let _ = self.reg_tx.send_blocking(item);
            }

            fn register_status_notifier_host(&self, _service: String) {}

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
    };
}

define_status_notifier_watcher!(StatusNotifierWatcher, "org.freedesktop.StatusNotifierWatcher");
define_status_notifier_watcher!(StatusNotifierWatcherKde, "org.kde.StatusNotifierWatcher");

// --- Service ---

pub struct TrayService;

async fn fetch_item_data(proxy: &StatusNotifierItemProxy<'_>) -> (String, String, String, String, String) {
    (
        proxy.id().await.unwrap_or_default(),
        proxy.title().await.unwrap_or_default(),
        proxy.icon_name().await.unwrap_or_default(),
        proxy.attention_icon_name().await.unwrap_or_default(),
        proxy.status().await.unwrap_or_else(|_| "Active".to_string()),
    )
}

async fn fetch_and_emit(
    proxy: &StatusNotifierItemProxy<'_>,
    bus_name: &str,
    event_tx: &Sender<ItemEvent>,
) {
    let (_id, title, icon_name, attention_icon_name, status) = fetch_item_data(proxy).await;

    let _ = event_tx.send(ItemEvent::Updated {
        bus_name: bus_name.to_string(),
        title,
        icon_name,
        attention_icon_name,
        status,
    }).await;
}

async fn add_item(
    connection: &Connection,
    item_id: String,
    item_proxies: &mut HashMap<String, StatusNotifierItemProxy<'static>>,
    tray_data: &mut TrayData,
    event_tx: &Sender<ItemEvent>,
    data_tx: &Sender<TrayData>,
) {
    if item_proxies.contains_key(&item_id) { return; }

    // Parse destination + path: could be "bus.name" (path defaults to /StatusNotifierItem)
    // or "bus.name/object/path" (combined from object path registration)
    let (dest, path) = if item_id.contains('/') {
        let slash = item_id.find('/').unwrap();
        (item_id[..slash].to_string(), item_id[slash..].to_string())
    } else {
        (item_id.clone(), "/StatusNotifierItem".to_string())
    };

    info!("[tray] Adding tray item: {dest} {path}");

    let proxy = match StatusNotifierItemProxy::builder(connection)
        .destination(dest)
        .and_then(|b| b.path(path))
    {
        Ok(b) => match b.build().await {
            Ok(p) => p,
            Err(e) => {
                error!("[tray] Failed to create proxy for {item_id}: {e}");
                return;
            }
        },
        Err(e) => {
            error!("[tray] Failed to create proxy builder for {item_id}: {e}");
            return;
        }
    };

    let (id, title, icon_name, attention_icon_name, status) = fetch_item_data(&proxy).await;

    info!("[tray] Item details: id={id}, title={title}, icon={icon_name}, status={status}");

    item_proxies.insert(item_id.clone(), proxy.clone());

    tray_data.items.push(TrayItem {
        bus_name: item_id.clone(),
        id,
        title,
        icon_name,
        attention_icon_name,
        status,
    });
    let _ = data_tx.send(tray_data.clone()).await;

    // Spawn per-item signal watcher
    let event_tx_c = event_tx.clone();
    let bus_name_c = item_id.clone();
    tokio::spawn(async move {
        let mut icon_changed = proxy.receive_new_icon().await.ok();
        let mut title_changed = proxy.receive_new_title().await.ok();
        let mut status_changed = proxy.receive_new_status().await.ok();

        let has_any_signal = icon_changed.is_some() || title_changed.is_some() || status_changed.is_some();

        if has_any_signal {
            loop {
                let triggered = tokio::select! {
                    Some(_) = async { icon_changed.as_mut()?.next().await }, if icon_changed.is_some() => true,
                    Some(_) = async { title_changed.as_mut()?.next().await }, if title_changed.is_some() => true,
                    Some(_) = async { status_changed.as_mut()?.next().await }, if status_changed.is_some() => true,
                    else => break,
                };

                if triggered {
                    fetch_and_emit(&proxy, &bus_name_c, &event_tx_c).await;
                }
            }

            let _ = event_tx_c.send(ItemEvent::Disconnected(bus_name_c)).await;
        }
    });
}

impl Service for TrayService {
    type Data = TrayData;
    type Cmd = TrayCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(64);
        let (cmd_tx, cmd_rx) = bounded(32);

        tokio::spawn(async move {
            // --- Single connection for everything ---
            let connection = match Connection::session().await {
                Ok(c) => {
                    info!("[tray] Connected to session bus");
                    c
                },
                Err(e) => {
                    error!("[tray] Failed to connect to session bus: {e}");
                    return;
                }
            };

            // --- Register Watcher Interface on this connection ---
            let (reg_tx, reg_rx) = bounded::<String>(64);
            let watcher = StatusNotifierWatcher { reg_tx };

            if let Err(e) = connection
                .object_server()
                .at("/StatusNotifierWatcher", watcher)
                .await
            {
                error!("[tray] Failed to serve watcher interface: {e}");
                return;
            }

            // Request watcher bus names
            match connection.request_name("org.freedesktop.StatusNotifierWatcher").await {
                Ok(_) => info!("[tray] Acquired org.freedesktop.StatusNotifierWatcher"),
                Err(e) => error!("[tray] Failed to acquire watcher name: {e}"),
            }
            let _ = connection.request_name("org.kde.StatusNotifierWatcher").await;

            // Host bus name
            let pid = std::process::id();
            let host_name = format!("org.freedesktop.StatusNotifierHost-{pid}");
            match WellKnownName::try_from(host_name.clone()) {
                Ok(name) => match connection.request_name(name).await {
                    Ok(_) => info!("[tray] Acquired {host_name}"),
                    Err(e) => error!("[tray] Failed to acquire host name: {e}"),
                },
                Err(e) => error!("[tray] Invalid host name: {e}"),
            }

            // --- NameOwnerChanged monitoring on same connection ---
            let dbus_proxy = match DBusProxy::new(&connection).await {
                Ok(p) => p,
                Err(e) => {
                    error!("[tray] Failed to create DBus proxy: {e}");
                    return;
                }
            };
            let mut name_changed = match dbus_proxy.receive_name_owner_changed().await {
                Ok(s) => s,
                Err(e) => {
                    error!("[tray] Failed to subscribe to name changes: {e}");
                    return;
                }
            };

            // --- Scan for already-existing items ---
            let mut existing_names: Vec<String> = Vec::new();
            match dbus_proxy.list_names().await {
                Ok(names) => {
                    for name in &names {
                        if name.starts_with("org.freedesktop.StatusNotifierItem-") {
                            existing_names.push(name.to_string());
                        }
                    }
                },
                Err(e) => error!("[tray] Failed to list names: {e}"),
            }
            info!("[tray] Found {} existing tray items", existing_names.len());

            // Event channel from per-item tasks
            let (event_tx, event_rx) = async_channel::unbounded::<ItemEvent>();

            // Track active items
            let mut item_proxies: HashMap<String, StatusNotifierItemProxy<'static>> = HashMap::new();
            let mut tray_data = TrayData::default();

            // Add existing items
            for name in existing_names {
                add_item(&connection, name, &mut item_proxies, &mut tray_data, &event_tx, &data_tx).await;
            }

            let mut cmd_rx = Box::pin(cmd_rx);

            info!("[tray] Ready, listening for registrations");

            loop {
                tokio::select! {
                    // New item registered via D-Bus
                    Ok(bus_name) = reg_rx.recv() => {
                        add_item(&connection, bus_name, &mut item_proxies, &mut tray_data, &event_tx, &data_tx).await;
                    }

                    // Item lost owner
                    Some(signal) = name_changed.next() => {
                        if let Ok(args) = signal.args() {
                            let name = &*args.name;
                            if name.starts_with("org.freedesktop.StatusNotifierItem-") && args.new_owner().is_none() {
                                info!("[tray] Item disconnected: {name}");
                                item_proxies.remove(name);
                                tray_data.items.retain(|i| i.bus_name != name);
                                let _ = data_tx.send(tray_data.clone()).await;
                            }
                        }
                    }

                    // Event from per-item task
                    Ok(event) = event_rx.recv() => {
                        match event {
                            ItemEvent::Updated { bus_name, title, icon_name, attention_icon_name, status } => {
                                if let Some(item) = tray_data.items.iter_mut().find(|i| i.bus_name == bus_name) {
                                    item.title = title;
                                    item.icon_name = icon_name;
                                    item.attention_icon_name = attention_icon_name;
                                    item.status = status;
                                }
                                let _ = data_tx.send(tray_data.clone()).await;
                            }
                            ItemEvent::Disconnected(bus_name) => {
                                warn!("[tray] Item task disconnected: {bus_name}");
                                item_proxies.remove(&bus_name);
                                tray_data.items.retain(|i| i.bus_name != bus_name);
                                let _ = data_tx.send(tray_data.clone()).await;
                            }
                        }
                    }

                    // Commands from UI
                    Some(cmd) = cmd_rx.next() => {
                        match cmd {
                            TrayCmd::Activate(ref bn) => {
                                info!("[tray] Activate: {bn}");
                                if let Some(p) = item_proxies.get(bn) {
                                    if let Err(e) = p.activate(0, 0).await {
                                        error!("[tray] Activate failed: {e}");
                                    }
                                } else {
                                    warn!("[tray] Activate: item not found: {bn}");
                                }
                            }
                            TrayCmd::ContextMenu(ref bn) => {
                                info!("[tray] ContextMenu: {bn}");
                                if let Some(p) = item_proxies.get(bn) {
                                    if let Err(e) = p.context_menu(0, 0).await {
                                        error!("[tray] ContextMenu failed: {e}");
                                    }
                                }
                            }
                            TrayCmd::SecondaryActivate(ref bn) => {
                                info!("[tray] SecondaryActivate: {bn}");
                                if let Some(p) = item_proxies.get(bn) {
                                    if let Err(e) = p.secondary_activate(0, 0).await {
                                        error!("[tray] SecondaryActivate failed: {e}");
                                    }
                                }
                            }
                            TrayCmd::Scroll(ref bn, delta, ref orientation) => {
                                if let Some(p) = item_proxies.get(bn) {
                                    let _ = p.scroll(delta, orientation).await;
                                }
                            }
                        }
                    }
                }
            }
        });

        (ServiceStore::new(data_rx, TrayData::default()), cmd_tx)
    }
}
