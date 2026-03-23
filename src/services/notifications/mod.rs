pub mod server;

use crate::services::notifications::server::{NotificationServer, NotificationCmd, NotificationServerSignals};
use async_channel::{bounded, Sender};
use log::{error, info};
use serde::Serialize;
use zbus::connection::Builder;
use zbus::object_server::InterfaceRef;
use crate::store::ServiceStore;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct NotificationAction {
    pub key: String,
    pub label: String,
}

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Serialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub urgency: u8,
    pub timestamp: i64,
    pub actions: Vec<NotificationAction>,
    #[serde(skip)]
    pub on_action: Option<HashMap<String, Arc<dyn Fn() + Send + Sync>>>,
    pub internal_id: u64,
}

impl Default for Notification {
    fn default() -> Self {
        Self {
            id: 0, app_name: String::new(), app_icon: String::new(),
            summary: String::new(), body: String::new(), urgency: 0,
            timestamp: 0, actions: Vec::new(), on_action: None, internal_id: 0,
        }
    }
}

impl PartialEq for Notification {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.app_name == other.app_name
            && self.app_icon == other.app_icon && self.summary == other.summary
            && self.body == other.body && self.urgency == other.urgency
            && self.timestamp == other.timestamp && self.actions == other.actions
            && self.internal_id == other.internal_id
    }
}

impl std::fmt::Debug for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notification")
            .field("id", &self.id)
            .field("app_name", &self.app_name)
            .field("summary", &self.summary)
            .field("internal_id", &self.internal_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct NotificationData {
    pub notifications: Vec<Notification>,
    pub last_id: u32,
}

pub struct NotificationService;

impl NotificationService {
    pub fn spawn_with_raw_tx() -> (ServiceStore<NotificationData>, Sender<NotificationCmd>, Sender<Notification>) {
        let (raw_tx, raw_rx) = bounded::<Notification>(64);
        let (data_tx, data_rx) = bounded::<NotificationData>(64);
        let (cmd_tx, cmd_rx) = bounded::<NotificationCmd>(32);
        let server_cmd_tx = cmd_tx.clone();
        let raw_tx_ret = raw_tx.clone();
        
        tokio::spawn(async move {
            let conn = async {
                let server = NotificationServer::new(raw_tx, server_cmd_tx);
                let builder = Builder::session()?;
                let builder = builder.name("org.freedesktop.Notifications")?;
                let builder = builder.serve_at("/org/freedesktop/Notifications", server)?;
                builder.build().await
            }.await;

            let conn = match conn {
                Ok(c) => c,
                Err(e) => {
                    error!("[notifications] Failed to register D-Bus service: {:?}", e);
                    return;
                }
            };

            info!("[notifications] D-Bus service registered");

            let interface_ref: InterfaceRef<NotificationServer> = match conn
                .object_server()
                .interface("/org/freedesktop/Notifications")
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    error!("[notifications] Failed to get interface ref: {:?}", e);
                    return;
                }
            };

            let mut history: Vec<Notification> = Vec::new();

            loop {
                tokio::select! {
                    Ok(n) = raw_rx.recv() => {
                        info!("[notifications] {} - {}", n.app_name, n.summary);
                        let id = n.id;
                        if let Some(pos) = history.iter().position(|x| x.id == id) {
                            history[pos] = n;
                        } else {
                            history.push(n);
                        }
                        if history.len() > 20 { history.remove(0); }
                        let _ = data_tx.send(NotificationData { 
                            notifications: history.clone(), 
                            last_id: id 
                        }).await;
                    }

                    Ok(cmd) = cmd_rx.recv() => {
                        match cmd {
                            NotificationCmd::Close(id) => {
                                info!("[notifications] Notification {id} closed");
                                history.retain(|n| n.id != id);
                                let _ = data_tx.send(NotificationData { 
                                    notifications: history.clone(), 
                                    last_id: 0 
                                }).await;
                                let _ = interface_ref.notification_closed(id, 2).await;
                            },
                            NotificationCmd::Action(id, key) => {
                                info!("[notifications] Notification {id} action: {key}");
                                let _ = interface_ref.action_invoked(id, &key).await;
                                history.retain(|n| n.id != id);
                                let _ = data_tx.send(NotificationData { 
                                    notifications: history.clone(), 
                                    last_id: 0 
                                }).await;
                                let _ = interface_ref.notification_closed(id, 2).await;
                            }
                        }
                    }
                }
            }
        });

        (ServiceStore::new(data_rx, Default::default()), cmd_tx, raw_tx_ret)
    }
}
