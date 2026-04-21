use std::collections::HashMap;
use std::path::PathBuf;

use super::Side;

/// Returns the base config directory for Axis (XDG_CONFIG_HOME/axis).
pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".config")
        });
    base.join("axis")
}

/// Returns the system hostname, falling back to "axis-device" if unavailable.
pub fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "axis-device".into())
}

/// Which edge of the local screen a peer is positioned at.
/// Mirrors `Side` from the protocol module for config persistence.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum KnownPeerArrangementSide {
    Left,
    #[default]
    Right,
    Top,
    Bottom,
}

impl From<Side> for KnownPeerArrangementSide {
    fn from(s: Side) -> Self {
        match s {
            Side::Left => Self::Left,
            Side::Right => Self::Right,
            Side::Top => Self::Top,
            Side::Bottom => Self::Bottom,
        }
    }
}

impl From<KnownPeerArrangementSide> for Side {
    fn from(s: KnownPeerArrangementSide) -> Self {
        match s {
            KnownPeerArrangementSide::Left => Self::Left,
            KnownPeerArrangementSide::Right => Self::Right,
            KnownPeerArrangementSide::Top => Self::Top,
            KnownPeerArrangementSide::Bottom => Self::Bottom,
        }
    }
}

/// Complete persisted state for a known (paired) peer.
/// This is the single source of truth for all peer configuration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KnownPeer {
    pub device_id: String,
    pub device_name: String,
    pub hostname: String,
    pub address: String,
    pub address_v6: Option<String>,
    pub trusted: bool,
    pub clipboard: bool,
    pub audio: bool,
    pub drag_drop: bool,
    pub arrangement_side: KnownPeerArrangementSide,
    pub arrangement_x: i32,
    pub arrangement_y: i32,
}

impl Default for KnownPeer {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            device_name: String::new(),
            hostname: String::new(),
            address: String::new(),
            address_v6: None,
            trusted: false,
            clipboard: true,
            audio: false,
            drag_drop: false,
            arrangement_side: KnownPeerArrangementSide::Right,
            arrangement_x: 0,
            arrangement_y: 0,
        }
    }
}

impl KnownPeer {
    /// Build a `PeerConfig` from this persisted state.
    pub fn to_peer_config(&self) -> super::PeerConfig {
        let offset = match self.arrangement_side {
            KnownPeerArrangementSide::Left | KnownPeerArrangementSide::Right => self.arrangement_y,
            KnownPeerArrangementSide::Top | KnownPeerArrangementSide::Bottom => self.arrangement_x,
        };
        super::PeerConfig {
            trusted: self.trusted,
            arrangement: super::PeerArrangement {
                side: self.arrangement_side.into(),
                offset,
            },
            clipboard: self.clipboard,
            audio: self.audio,
            drag_drop: self.drag_drop,
            version: 0, // version is runtime-only
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct KnownPeersStore {
    pub peers: HashMap<String, KnownPeer>,
}

pub fn known_peers_path() -> PathBuf {
    config_dir().join("continuity").join("known_peers.json")
}

pub fn load_known_peers() -> KnownPeersStore {
    let path = known_peers_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(store) = serde_json::from_str(&content) {
            return store;
        }
    }
    KnownPeersStore::default()
}

pub fn save_known_peers(store: &KnownPeersStore) {
    let path = known_peers_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = std::fs::write(&path, json);
    }
}
