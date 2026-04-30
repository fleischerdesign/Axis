use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub address: SocketAddr,
    pub status: PeerStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Disconnected,
    Connecting,
    Connected,
    Pairing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContinuityMessage {
    Hello { device_id: String, device_name: String },
    Heartbeat,
    ClipboardUpdate { content: Vec<u8>, mime_type: String },
}
