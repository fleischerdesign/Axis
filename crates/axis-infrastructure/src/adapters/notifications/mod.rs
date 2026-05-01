use axis_domain::models::notifications::{Notification, NotificationAction, NotificationStatus};
use axis_domain::ports::notifications::{ActionHandler, NotificationError, NotificationProvider, NotificationStream};
use async_trait::async_trait;
use log::info;
use tokio::sync::{watch, mpsc};
use tokio_stream::wrappers::WatchStream;
use std::sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}};
use std::collections::HashMap;
use zbus::connection;
use zbus::zvariant::Value;

const MAX_HISTORY: usize = 20;

enum Cmd {
    Show(Notification),
    Close(u32),
    Action(u32, String),
}

struct NotificationsIface {
    cmd_tx: mpsc::Sender<Cmd>,
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl NotificationsIface {
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, Value>,
        _expire_timeout: i32,
    ) -> u32 {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);

        let id = if replaces_id == 0 {
            NEXT_ID.fetch_add(1, Ordering::SeqCst)
        } else {
            replaces_id
        };

        let urgency = hints
            .get("urgency")
            .and_then(|v| v.downcast_ref::<u8>().ok())
            .unwrap_or(1);

        let parsed_actions = actions
            .chunks_exact(2)
            .map(|c| NotificationAction {
                key: c[0].clone(),
                label: c[1].clone(),
            })
            .collect();

        let n = Notification {
            id,
            app_name,
            app_icon,
            summary,
            body,
            urgency,
            actions: parsed_actions,
            timeout: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            internal_id: 0,
            ignore_dnd: urgency == 2,
        };

        let _ = self.cmd_tx.try_send(Cmd::Show(n));
        id
    }

    fn close_notification(&self, id: u32) {
        let _ = self.cmd_tx.try_send(Cmd::Close(id));
    }

    #[zbus(signal)]
    async fn action_invoked(emitter: &zbus::object_server::SignalEmitter<'_>, id: u32, action_key: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn notification_closed(emitter: &zbus::object_server::SignalEmitter<'_>, id: u32, reason: u32) -> zbus::Result<()>;

    fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".into(),
            "actions".into(),
            "icon-static".into(),
            "persistence".into(),
        ]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "axis-shell-notifications".into(),
            "AXIS Project".into(),
            "0.1.0".into(),
            "1.2".into(),
        )
    }
}

pub struct ZbusNotificationProvider {
    status_tx: watch::Sender<NotificationStatus>,
    cmd_tx: mpsc::Sender<Cmd>,
    action_handlers: Arc<Mutex<HashMap<(u32, String), ActionHandler>>>,
}

impl ZbusNotificationProvider {
    pub async fn new() -> Result<Arc<Self>, NotificationError> {
        let (status_tx, _) = watch::channel(NotificationStatus::default());
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(64);
        let action_handlers = Arc::new(Mutex::new(HashMap::new()));

        let server = NotificationsIface {
            cmd_tx: cmd_tx.clone(),
        };

        let conn = connection::Builder::session()
            .map_err(|e| NotificationError::ProviderError(e.to_string()))?
            .name("org.freedesktop.Notifications")
            .map_err(|e| NotificationError::ProviderError(e.to_string()))?
            .serve_at("/org/freedesktop/Notifications", server)
            .map_err(|e| NotificationError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| NotificationError::ProviderError(e.to_string()))?;

        info!("[notifications] D-Bus service registered on org.freedesktop.Notifications");

        let iface_ref: zbus::object_server::InterfaceRef<NotificationsIface> = conn
            .object_server()
            .interface("/org/freedesktop/Notifications")
            .await
            .map_err(|e| NotificationError::ProviderError(e.to_string()))?;

        let status_tx_bg = status_tx.clone();
        let cmd_tx_bg = cmd_tx.clone();
        let action_handlers_bg = action_handlers.clone();

        tokio::spawn(async move {
            let mut history: Vec<Notification> = Vec::new();

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    Cmd::Show(n) => {
                        info!("[notifications] {} - {}", n.app_name, n.summary);
                        let id = n.id;
                        let timeout = n.timeout;
                        if let Some(pos) = history.iter().position(|x| x.id == id) {
                            history[pos] = n;
                        } else {
                            history.push(n);
                        }
                        if history.len() > MAX_HISTORY {
                            history.remove(0);
                        }
                        let _ = status_tx_bg.send(NotificationStatus {
                            notifications: history.clone(),
                            last_id: id,
                        });

                        if timeout > 0 {
                            let tx = cmd_tx_bg.clone();
                            let ah = action_handlers_bg.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(timeout as u64)).await;
                                ah.lock().unwrap().retain(|(nid, _), _: &mut ActionHandler| *nid != id);
                                let _ = tx.send(Cmd::Close(id)).await;
                            });
                        }
                    }
                    Cmd::Close(id) => {
                        info!("[notifications] Notification {id} closed");
                        history.retain(|n| n.id != id);
                        let _ = status_tx_bg.send(NotificationStatus {
                            notifications: history.clone(),
                            last_id: 0,
                        });

                        let emitter = iface_ref.signal_emitter();
                        let _ = NotificationsIface::notification_closed(emitter, id, 2).await;
                        action_handlers_bg.lock().unwrap().retain(|(nid, _), _: &mut ActionHandler| *nid != id);
                    }
                    Cmd::Action(id, key) => {
                        info!("[notifications] Notification {id} action: {key}");

                        {
                            let handlers = action_handlers_bg.lock().unwrap();
                            if let Some(handler) = handlers.get(&(id, key.clone())) {
                                handler();
                            }
                        }

                        let emitter = iface_ref.signal_emitter();
                        let _ = NotificationsIface::action_invoked(emitter, id, &key).await;
                        let _ = NotificationsIface::notification_closed(emitter, id, 2).await;
                        history.retain(|n| n.id != id);
                        let _ = status_tx_bg.send(NotificationStatus {
                            notifications: history.clone(),
                            last_id: 0,
                        });
                        action_handlers_bg.lock().unwrap().retain(|(nid, _), _: &mut ActionHandler| *nid != id);
                    }
                }
            }
        });

        Ok(Arc::new(Self {
            status_tx,
            cmd_tx,
            action_handlers,
        }))
    }
}

#[async_trait]
impl NotificationProvider for ZbusNotificationProvider {
    async fn get_status(&self) -> Result<NotificationStatus, NotificationError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<NotificationStream, NotificationError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn close_notification(&self, id: u32) -> Result<(), NotificationError> {
        self.cmd_tx
            .send(Cmd::Close(id))
            .await
            .map_err(|e| NotificationError::ProviderError(e.to_string()))
    }

    async fn invoke_action(&self, id: u32, action_key: &str) -> Result<(), NotificationError> {
        self.cmd_tx
            .send(Cmd::Action(id, action_key.to_string()))
            .await
            .map_err(|e| NotificationError::ProviderError(e.to_string()))
    }

    async fn show(
        &self,
        notification: Notification,
        action_handlers: HashMap<String, ActionHandler>,
    ) -> Result<u32, NotificationError> {
        let id = notification.id;
        {
            let mut handlers = self.action_handlers.lock().unwrap();
            for (key, handler) in action_handlers {
                handlers.insert((id, key), handler);
            }
        }
        self.cmd_tx
            .send(Cmd::Show(notification))
            .await
            .map_err(|e| NotificationError::ProviderError(e.to_string()))?;
        Ok(id)
    }
}
