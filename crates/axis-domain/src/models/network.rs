use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessPoint {
    pub id: String,
    pub ssid: String,
    pub strength: u8,
    pub is_active: bool,
    pub needs_auth: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub is_wifi_connected: bool,
    pub is_ethernet_connected: bool,
    pub is_wifi_enabled: bool,
    pub active_strength: u8,
    pub is_scanning: bool,
    pub access_points: Vec<AccessPoint>,
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self {
            is_wifi_connected: false,
            is_ethernet_connected: false,
            is_wifi_enabled: false,
            active_strength: 0,
            is_scanning: false,
            access_points: vec![],
        }
    }
}
