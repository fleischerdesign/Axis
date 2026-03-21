pub mod item_proxy;

use super::Service;
use crate::store::ServiceStore;
use async_channel::{bounded, Sender};
use futures_util::StreamExt;
use item_proxy::StatusNotifierItemProxy;
use std::collections::HashMap;
use zbus::connection::Builder;
use zbus::interface;
use zbus::fdo::DBusProxy;
use zbus::Connection;
use zbus::zvariant::Value;
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

struct StatusNotifierWatcher {
    reg_tx: Sender<String>,
}

#[interface(name = "org.freedesktop.StatusNotifierWatcher")]
impl StatusNotifierWatcher {
    fn register_status_notifier_item(&self, service: String) {
        eprintln!("[TrayService] RegisterStatusNotifierItem: {service}");
        let _ = self.reg_tx.send_blocking(service);
    }

    fn register_status_notifier_host(&self, _service: String) {
        eprintln!("[TrayService] RegisterStatusNotifierHost: {_service}");
    }

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

// --- Service ---

pub struct TrayService;

async fn fetch_and_emit(
    proxy: &StatusNotifierItemProxy<'_>,
    bus_name: &str,
    event_tx: &Sender<ItemEvent>,
) {
    let _id = proxy.id().await.unwrap_or_default();
    let title = proxy.title().await.unwrap_or_default();
    let icon_name = proxy.icon_name().await.unwrap_or_default();
    let attention_icon_name = proxy.attention_icon_name().await.unwrap_or_default();
    let status = proxy.status().await.unwrap_or_else(|_| "Active".to_string());

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
    bus_name: String,
    item_proxies: &mut HashMap<String, StatusNotifierItemProxy<'static>>,
    tray_data: &mut TrayData,
    event_tx: &Sender<ItemEvent>,
    data_tx: &Sender<TrayData>,
) {
    if item_proxies.contains_key(&bus_name) { return; }

    eprintln!("[TrayService] Adding tray item: {bus_name}");

    let proxy = match StatusNotifierItemProxy::builder(connection)
        .destination(bus_name.clone())
        .and_then(|b| b.path("/StatusNotifierItem"))
    {
        Ok(b) => match b.build().await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[TrayService] Failed to create proxy for {bus_name}: {e}");
                return;
            }
        },
        Err(e) => {
            eprintln!("[TrayService] Failed to create proxy builder for {bus_name}: {e}");
            return;
        }
    };

    let (id, title, icon_name, attention_icon_name, status) = (
        proxy.id().await.unwrap_or_default(),
        proxy.title().await.unwrap_or_default(),
        proxy.icon_name().await.unwrap_or_default(),
        proxy.attention_icon_name().await.unwrap_or_default(),
        proxy.status().await.unwrap_or_else(|_| "Active".to_string()),
    );

    eprintln!("[TrayService] Item details: id={id}, title={title}, icon={icon_name}, status={status}");

    item_proxies.insert(bus_name.clone(), proxy.clone());

    tray_data.items.push(TrayItem {
        bus_name: bus_name.clone(),
        id,
        title,
        icon_name,
        attention_icon_name,
        status,
    });
    let _ = data_tx.send(tray_data.clone()).await;

    // Spawn per-item signal watcher
    let event_tx_c = event_tx.clone();
    let bus_name_c = bus_name.clone();
    tokio::spawn(async move {
        let mut icon_changed = proxy.receive_new_icon().await.ok();
        let mut title_changed = proxy.receive_new_title().await.ok();
        let mut status_changed = proxy.receive_new_status().await.ok();

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
                    eprintln!("[TrayService] Connected to session bus");
                    c
                },
                Err(e) => {
                    eprintln!("[TrayService] Failed to connect to session bus: {e}");
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
                eprintln!("[TrayService] Failed to serve watcher interface: {e}");
                return;
            }

            // Request watcher bus names
            match connection.request_name("org.freedesktop.StatusNotifierWatcher").await {
                Ok(_) => eprintln!("[TrayService] Acquired org.freedesktop.StatusNotifierWatcher"),
                Err(e) => eprintln!("[TrayService] Failed to acquire watcher name: {e}"),
            }
            let _ = connection.request_name("org.kde.StatusNotifierWatcher").await;

            // Host bus name
            let pid = std::process::id();
            let host_name = format!("org.freedesktop.StatusNotifierHost-{pid}");
            match WellKnownName::try_from(host_name.clone()) {
                Ok(name) => match connection.request_name(name).await {
                    Ok(_) => eprintln!("[TrayService] Acquired {host_name}"),
                    Err(e) => eprintln!("[TrayService] Failed to acquire host name: {e}"),
                },
                Err(e) => eprintln!("[TrayService] Invalid host name: {e}"),
            }

            // --- NameOwnerChanged monitoring on same connection ---
            let dbus_proxy = match DBusProxy::new(&connection).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[TrayService] Failed to create DBus proxy: {e}");
                    return;
                }
            };
            let mut name_changed = match dbus_proxy.receive_name_owner_changed().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[TrayService] Failed to subscribe to name changes: {e}");
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
                Err(e) => eprintln!("[TrayService] Failed to list names: {e}"),
            }
            eprintln!("[TrayService] Found {} existing tray items", existing_names.len());

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

            eprintln!("[TrayService] Ready, listening for registrations");

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
                                eprintln!("[TrayService] Item disconnected: {name}");
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
                                eprintln!("[TrayService] Item task disconnected: {bus_name}");
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
                                if let Some(p) = item_proxies.get(bn) {
                                    let _ = p.activate(0, 0).await;
                                }
                            }
                            TrayCmd::ContextMenu(ref bn) => {
                                if let Some(p) = item_proxies.get(bn) {
                                    let _ = p.context_menu(0, 0).await;
                                }
                            }
                            TrayCmd::SecondaryActivate(ref bn) => {
                                if let Some(p) = item_proxies.get(bn) {
                                    let _ = p.secondary_activate(0, 0).await;
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
