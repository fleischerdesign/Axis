use std::collections::HashMap;
use std::path::PathBuf;

/// Returns the base config directory for Axis (XDG_CONFIG_HOME/axis).
pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME environment variable must be set");
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KnownPeer {
    pub device_id: String,
    pub device_name: String,
    pub hostname: String,
    pub address: String,
    pub address_v6: Option<String>,
    pub trusted: bool,
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
