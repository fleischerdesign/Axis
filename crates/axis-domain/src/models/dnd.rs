use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DndStatus {
    pub enabled: bool,
}

impl Default for DndStatus {
    fn default() -> Self {
        Self { enabled: false }
    }
}
