use axis_domain::models::bluetooth::{BluetoothDevice, BluetoothStatus};
use axis_domain::ports::bluetooth::{BluetoothStream, BluetoothProvider, BluetoothError};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Mutex;

pub struct MockBluetoothProvider {
    status: Mutex<BluetoothStatus>,
    status_tx: watch::Sender<BluetoothStatus>,
}

impl MockBluetoothProvider {
    pub fn new() -> Self {
        let initial = BluetoothStatus {
            powered: true,
            is_scanning: false,
            devices: vec![
                BluetoothDevice {
                    id: "1".to_string(),
                    name: Some("Mock Headphones".to_string()),
                    connected: false,
                    paired: true,
                    icon: "audio-headphones-symbolic".to_string(),
                },
                BluetoothDevice {
                    id: "2".to_string(),
                    name: Some("Mock Phone".to_string()),
                    connected: true,
                    paired: true,
                    icon: "phone-symbolic".to_string(),
                },
            ],
        };
        let (tx, _) = watch::channel(initial.clone());
        Self {
            status: Mutex::new(initial),
            status_tx: tx,
        }
    }
}

#[async_trait]
impl BluetoothProvider for MockBluetoothProvider {
    async fn get_status(&self) -> Result<BluetoothStatus, BluetoothError> {
        Ok(self.status.lock().unwrap().clone())
    }

    async fn subscribe(&self) -> Result<BluetoothStream, BluetoothError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn connect(&self, id: &str) -> Result<(), BluetoothError> {
        let mut status = self.status.lock().unwrap();
        if let Some(device) = status.devices.iter_mut().find(|d| d.id == id) {
            device.connected = true;
            let _ = self.status_tx.send(status.clone());
            Ok(())
        } else {
            Err(BluetoothError::DeviceNotFound(id.to_string()))
        }
    }

    async fn disconnect(&self, id: &str) -> Result<(), BluetoothError> {
        let mut status = self.status.lock().unwrap();
        if let Some(device) = status.devices.iter_mut().find(|d| d.id == id) {
            device.connected = false;
            let _ = self.status_tx.send(status.clone());
            Ok(())
        } else {
            Err(BluetoothError::DeviceNotFound(id.to_string()))
        }
    }

    async fn set_powered(&self, powered: bool) -> Result<(), BluetoothError> {
        let mut status = self.status.lock().unwrap();
        status.powered = powered;
        let _ = self.status_tx.send(status.clone());
        Ok(())
    }

    async fn start_scan(&self) -> Result<(), BluetoothError> {
        let mut status = self.status.lock().unwrap();
        status.is_scanning = true;
        let _ = self.status_tx.send(status.clone());
        Ok(())
    }

    async fn stop_scan(&self) -> Result<(), BluetoothError> {
        let mut status = self.status.lock().unwrap();
        status.is_scanning = false;
        let _ = self.status_tx.send(status.clone());
        Ok(())
    }
}
