pub mod server;

use crate::services::notifications::server::{NotificationServer, NotificationCmd};
use async_channel::{bounded, Receiver, Sender};
use serde::Serialize;
use zbus::connection::Builder;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub urgency: u8,
    pub timestamp: i64,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct NotificationData {
    pub notifications: Vec<Notification>,
    /// Hilft der UI zu erkennen, ob gerade eine NEUE Nachricht reinkam
    pub last_id: u32,
}

pub struct NotificationService;

impl NotificationService {
    pub fn spawn() -> (Receiver<NotificationData>, Sender<NotificationCmd>) {
        let (raw_tx, raw_rx) = bounded::<Notification>(64);
        let (data_tx, data_rx) = bounded::<NotificationData>(64);
        let (cmd_tx, cmd_rx) = bounded(32);
        
        let server = NotificationServer::new(raw_tx, cmd_rx.clone());

        tokio::spawn(async move {
            let _conn = Builder::session()
                .unwrap()
                .name("org.freedesktop.Notifications")
                .unwrap()
                .serve_at("/org/freedesktop/Notifications", server)
                .unwrap()
                .build()
                .await
                .unwrap();

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });

        // Mapper: History Management
        tokio::spawn(async move {
            let mut history: Vec<Notification> = Vec::new();
            while let Ok(n) = raw_rx.recv().await {
                let id = n.id;
                history.push(n);
                
                // Limit: Nur die letzten 20 behalten (Clean & Memory Safe)
                if history.len() > 20 {
                    history.remove(0);
                }

                let _ = data_tx.send(NotificationData { 
                    notifications: history.clone(),
                    last_id: id 
                }).await;
            }
        });

        (data_rx, cmd_tx)
    }
}
