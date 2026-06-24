use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Urgency {
    #[default]
    Normal = 0,
    Low = 1,
    Critical = 2,
}

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
    pub urgency: Urgency,
    pub actions: Vec<NotificationAction>,
    pub timeout: u32,
    pub timestamp: i64,
    pub internal_id: u64,
    pub ignore_dnd: bool,
    pub input_placeholder: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgency_default_is_normal() {
        assert_eq!(Urgency::default(), Urgency::Normal);
    }

    #[test]
    fn notification_action_serde_roundtrip() {
        let a = NotificationAction {
            key: "default".into(),
            label: "Open".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let back: NotificationAction = serde_json::from_str(&json).unwrap();
        assert_eq!(a.label, back.label);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotificationStatus {
    pub notifications: Vec<Notification>,
    pub last_id: u32,
}
