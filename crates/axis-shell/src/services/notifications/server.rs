use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use zbus::interface;
use zbus::zvariant::Value;
use crate::services::notifications::{Notification, NotificationAction};
use async_channel::Sender;
use zbus::object_server::SignalEmitter;

#[derive(Debug, Clone)]
pub enum NotificationCmd {
    Show(Notification),
    Close(u32),
    Action(u32, String),
}

pub struct NotificationServer {
    cmd_tx: Sender<NotificationCmd>,
}

impl NotificationServer {
    pub fn new(cmd_tx: Sender<NotificationCmd>) -> Self {
        Self { cmd_tx }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
    pub fn notify(
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
        static NOTIFICATION_ID: AtomicU32 = AtomicU32::new(1);
        
        let id = if replaces_id == 0 {
            NOTIFICATION_ID.fetch_add(1, Ordering::SeqCst)
        } else {
            replaces_id
        };

        let urgency = hints.get("urgency")
            .and_then(|v| v.downcast_ref::<u8>().ok())
            .unwrap_or(1);

        let mut parsed_actions = Vec::new();
        for chunk in actions.chunks_exact(2) {
            parsed_actions.push(NotificationAction {
                key: chunk[0].clone(),
                label: chunk[1].clone(),
            });
        }

        let notification = Notification {
            id,
            app_name,
            app_icon,
            summary,
            body,
            urgency,
            timestamp: chrono::Local::now().timestamp(),
            actions: parsed_actions,
            on_action: None,
            internal_id: 0,
            timeout: 0,
        };

        let _ = self.cmd_tx.send_blocking(NotificationCmd::Show(notification));
        id
    }

    pub fn close_notification(&self, id: u32) {
        let _ = self.cmd_tx.send_blocking(NotificationCmd::Close(id));
    }

    #[zbus(signal)]
    async fn action_invoked(emitter: &SignalEmitter<'_>, id: u32, action_key: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn notification_closed(emitter: &SignalEmitter<'_>, id: u32, reason: u32) -> zbus::Result<()>;

    pub fn get_capabilities(&self) -> Vec<String> {
        vec!["body".to_string(), "actions".to_string(), "icon-static".to_string(), "persistence".to_string()]
    }

    pub fn get_server_information(&self) -> (String, String, String, String) {
        ("axis-shell-notifications".to_string(), "AXIS Project".to_string(), "0.1.0".to_string(), "1.2".to_string())
    }
}
