use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SshSession {
    pub pid: u32,
    pub username: String,
    pub terminal: String,
    pub source_ip: Option<String>,
    pub connected_for: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SshStatus {
    pub sessions: Vec<SshSession>,
    pub active_count: usize,
}

impl Default for SshStatus {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            active_count: 0,
        }
    }
}
