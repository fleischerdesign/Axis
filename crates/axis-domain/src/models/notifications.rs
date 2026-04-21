use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NotificationAction {
    pub key: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub urgency: u8,
    pub actions: Vec<NotificationAction>,
    pub timeout: u32,
    pub timestamp: i64,
    pub internal_id: u64,
    pub ignore_dnd: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotificationStatus {
    pub notifications: Vec<Notification>,
    pub last_id: u32,
}
