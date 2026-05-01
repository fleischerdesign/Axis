use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothDevice {
    pub id: String,
    pub name: Option<String>,
    pub connected: bool,
    pub paired: bool,
    pub icon: String,
}

impl Default for BluetoothDevice {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: None,
            connected: false,
            paired: false,
            icon: "bluetooth-symbolic".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PairingType {
    Confirmation,
    PinCode,
    Passkey,
    Authorization,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingPairing {
    pub device_path: String,
    pub device_name: String,
    pub passkey: Option<String>,
    pub pairing_type: PairingType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothStatus {
    pub powered: bool,
    pub is_scanning: bool,
    pub devices: Vec<BluetoothDevice>,
    pub pending_pairing: Option<PendingPairing>,
}

impl Default for BluetoothStatus {
    fn default() -> Self {
        Self {
            powered: false,
            is_scanning: false,
            devices: vec![],
            pending_pairing: None,
        }
    }
}
