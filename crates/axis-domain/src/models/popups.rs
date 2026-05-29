use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PopupType {
    Launcher,
    Workspaces,
    Agenda,
    QuickSettings,
    Mpris,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PopupStatus {
    pub active_popup: Option<PopupType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_status_default_has_no_active_popup() {
        let s = PopupStatus { active_popup: None };
        assert!(s.active_popup.is_none());
    }

    #[test]
    fn popup_type_serde_roundtrip() {
        let types = vec![
            PopupType::Launcher,
            PopupType::Workspaces,
            PopupType::Agenda,
            PopupType::QuickSettings,
            PopupType::Mpris,
        ];
        for pt in types {
            let json = serde_json::to_string(&pt).unwrap();
            let back: PopupType = serde_json::from_str(&json).unwrap();
            assert_eq!(pt, back);
        }
    }
}
