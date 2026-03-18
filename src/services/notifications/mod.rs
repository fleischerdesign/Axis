pub mod server;

use crate::services::notifications::server::{NotificationServer, NotificationCmd, NotificationServerSignals};
use async_channel::{bounded, Receiver, Sender};
use serde::Serialize;
use zbus::connection::Builder;
use zbus::object_server::InterfaceRef;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct NotificationAction {
    pub key: String,
    pub label: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub urgency: u8,
    pub timestamp: i64,
    pub actions: Vec<NotificationAction>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct NotificationData {
    pub notifications: Vec<Notification>,
    pub last_id: u32,
}

pub struct NotificationService;

impl NotificationService {
    pub fn spawn() -> (Receiver<NotificationData>, Sender<NotificationCmd>) {
        let (raw_tx, raw_rx) = bounded::<Notification>(64);
        let (data_tx, data_rx) = bounded::<NotificationData>(64);
        let (cmd_tx, cmd_rx) = bounded::<NotificationCmd>(32);
        
        let cmd_rx_for_bus = cmd_rx.clone();
        let cmd_rx_for_mapper = cmd_rx.clone();

        tokio::spawn(async move {
            let server = NotificationServer::new(raw_tx);
            let conn = Builder::session()
                .unwrap()
                .name("org.freedesktop.Notifications")
                .unwrap()
                .serve_at("/org/freedesktop/Notifications", server)
                .unwrap()
                .build()
                .await
                .unwrap();

            let interface_ref: InterfaceRef<NotificationServer> = conn
                .object_server()
                .interface("/org/freedesktop/Notifications")
                .await
                .unwrap();

            // D-Bus Signal Loop (Signals auf InterfaceRef)
            while let Ok(cmd) = cmd_rx_for_bus.recv().await {
                match cmd {
                    NotificationCmd::Close(id) => {
                        let _ = interface_ref.notification_closed(id, 2).await;
                    },
                    NotificationCmd::Action(id, key) => {
                        let _ = interface_ref.action_invoked(id, &key).await;
                        let _ = interface_ref.notification_closed(id, 2).await;
                    }
                }
            }
        });

        // Mapper: History Management
        tokio::spawn(async move {
            let mut history: Vec<Notification> = Vec::new();
            
            loop {
                tokio::select! {
                    Ok(n) = raw_rx.recv() => {
                        let id = n.id;
                        if let Some(pos) = history.iter().position(|x| x.id == id) {
                            history[pos] = n;
                        } else {
                            history.push(n);
                        }
                        if history.len() > 20 { history.remove(0); }
                        let _ = data_tx.send(NotificationData { notifications: history.clone(), last_id: id }).await;
                    }
                    
                    Ok(cmd) = cmd_rx_for_mapper.recv() => {
                        if let NotificationCmd::Close(id) = cmd {
                            history.retain(|n| n.id != id);
                            let _ = data_tx.send(NotificationData { notifications: history.clone(), last_id: 0 }).await;
                        }
                    }
                }
            }
        });

        (data_rx, cmd_tx)
    }
}
