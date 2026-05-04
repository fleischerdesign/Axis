use axis_domain::models::notifications::{Notification, NotificationStatus};
use axis_domain::ports::notifications::{ActionHandler, NotificationProvider, NotificationError, NotificationStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MockNotificationProvider {
    status_tx: watch::Sender<NotificationStatus>,
}

impl MockNotificationProvider {
    pub fn new() -> Arc<Self> {
        let initial = NotificationStatus {
            notifications: vec![Notification {
                id: 1,
                app_name: "System".to_string(),
                app_icon: "system-software-update".to_string(),
                summary: "Update Available".to_string(),
                body: "A new version of Axis is ready.".to_string(),
                urgency: 1,
                actions: vec![],
                timeout: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                internal_id: 0,
                ignore_dnd: false,
                input_placeholder: None,
            }],
            last_id: 1,
        };
        let (status_tx, _) = watch::channel(initial);
        Arc::new(Self { status_tx })
    }
}

#[async_trait]
impl NotificationProvider for MockNotificationProvider {
    async fn get_status(&self) -> Result<NotificationStatus, NotificationError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<NotificationStream, NotificationError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn close_notification(&self, id: u32) -> Result<(), NotificationError> {
        self.status_tx.send_modify(|s| {
            s.notifications.retain(|n| n.id != id);
        });
        Ok(())
    }

    async fn invoke_action(&self, _id: u32, _action_key: &str, _user_input: Option<String>) -> Result<(), NotificationError> {
        Ok(())
    }

    async fn show(
        &self,
        notification: Notification,
        _action_handlers: HashMap<String, ActionHandler>,
    ) -> Result<u32, NotificationError> {
        let id = notification.id;
        self.status_tx.send_modify(|s| {
            if let Some(pos) = s.notifications.iter().position(|n| n.id == id) {
                s.notifications[pos] = notification;
            } else {
                s.notifications.push(notification);
            }
            s.last_id = id;
        });
        Ok(id)
    }
}
