use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use zbus::interface;
use zbus::zvariant::Value;
use crate::services::notifications::Notification;
use async_channel::{Sender, Receiver};

static NOTIFICATION_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub enum NotificationCmd {
    Close(u32),
    Action(u32, String),
}

pub struct NotificationServer {
    tx: Sender<Notification>,
    _cmd_rx: Receiver<NotificationCmd>,
}

impl NotificationServer {
    pub fn new(tx: Sender<Notification>, cmd_rx: Receiver<NotificationCmd>) -> Self {
        Self { tx, _cmd_rx: cmd_rx }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
    /// Die Hauptmethode für neue Benachrichtigungen
    pub fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        _actions: Vec<String>,
        hints: HashMap<String, Value>,
        _expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id == 0 {
            NOTIFICATION_ID.fetch_add(1, Ordering::SeqCst)
        } else {
            replaces_id
        };

        // Urgency aus Hints extrahieren (0=Low, 1=Normal, 2=Critical)
        let urgency = hints.get("urgency")
            .and_then(|v| v.downcast_ref::<u8>().ok())
            .unwrap_or(1);

        let notification = Notification {
            id,
            app_name,
            app_icon,
            summary,
            body,
            urgency,
            timestamp: chrono::Local::now().timestamp(),
        };

        // Nachricht an den Main-Thread schicken
        let _ = self.tx.send_blocking(notification);

        id
    }

    pub fn close_notification(&self, id: u32) {
        println!("Notification: Request to close ID {}", id);
    }

    pub fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "actions".to_string(),
            "icon-static".to_string(),
            "persistence".to_string(),
        ]
    }

    pub fn get_server_information(&self) -> (String, String, String, String) {
        (
            "carp-shell-notifications".to_string(),
            "Carp Project".to_string(),
            "0.1.0".to_string(),
            "1.2".to_string(),
        )
    }
}
