use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockStatus {
    pub is_locked: bool,
    pub is_supported: bool,
}

impl Default for LockStatus {
    fn default() -> Self {
        Self {
            is_locked: false,
            is_supported: false,
        }
    }
}
