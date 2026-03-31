use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct NotificationAction {
    pub key: String,
    pub label: String,
}

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
