use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdleInhibitStatus {
    pub inhibited: bool,
}
