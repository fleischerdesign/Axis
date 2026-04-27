use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PopupType {
    Launcher,
    Workspaces,
    Agenda,
    QuickSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PopupStatus {
    pub active_popup: Option<PopupType>,
}

impl Default for PopupStatus {
    fn default() -> Self {
        Self {
            active_popup: None,
        }
    }
}
