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
pub struct BluetoothStatus {
    pub powered: bool,
    pub is_scanning: bool,
    pub devices: Vec<BluetoothDevice>,
}

impl Default for BluetoothStatus {
    fn default() -> Self {
        Self {
            powered: false,
            is_scanning: false,
            devices: vec![],
        }
    }
}
