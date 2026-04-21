use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PopupType {
    Launcher,
    Workspaces,
    Calendar,
    QuickSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PopupStatus {
    pub active_popup: Option<PopupType>,
}
