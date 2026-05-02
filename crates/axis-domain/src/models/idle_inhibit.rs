use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdleInhibitStatus {
    pub inhibited: bool,
}

impl Default for IdleInhibitStatus {
    fn default() -> Self {
        Self { inhibited: false }
    }
}
