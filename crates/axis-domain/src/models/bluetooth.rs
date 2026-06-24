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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BluetoothStatus {
    pub powered: bool,
    pub is_scanning: bool,
    pub devices: Vec<BluetoothDevice>,
    pub pending_pairing: Option<PendingPairing>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bluetooth_device_default_icon() {
        let d = BluetoothDevice::default();
        assert_eq!(d.icon, "bluetooth-symbolic");
        assert!(!d.connected);
        assert!(!d.paired);
    }

    #[test]
    fn pending_pairing_serde_roundtrip() {
        let p = PendingPairing {
            device_path: "/org/bluez/hci0/dev_AA_BB".into(),
            device_name: "Headphones".into(),
            passkey: Some("123456".into()),
            pairing_type: PairingType::Passkey,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: PendingPairing = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
