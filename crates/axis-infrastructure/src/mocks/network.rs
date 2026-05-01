use axis_domain::models::network::{NetworkStatus, AccessPoint};
use axis_domain::ports::network::{NetworkProvider, NetworkError, NetworkStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::{Arc, Mutex};

pub struct MockNetworkProvider {
    status: Mutex<NetworkStatus>,
    status_tx: watch::Sender<NetworkStatus>,
}

impl MockNetworkProvider {
    pub fn new() -> Arc<Self> {
        let initial = NetworkStatus {
            is_wifi_connected: true,
            is_ethernet_connected: false,
            is_wifi_enabled: true,
            active_strength: 80,
            is_scanning: false,
            access_points: vec![
                AccessPoint {
                    id: "ap1".to_string(),
                    ssid: "Home WiFi".to_string(),
                    strength: 80,
                    is_active: true,
                    needs_auth: true,
                },
                AccessPoint {
                    id: "ap2".to_string(),
                    ssid: "Guest WiFi".to_string(),
                    strength: 40,
                    is_active: false,
                    needs_auth: false,
                },
            ],
        };
        let (tx, _) = watch::channel(initial.clone());
        Arc::new(Self {
            status: Mutex::new(initial),
            status_tx: tx,
        })
    }
}

#[async_trait]
impl NetworkProvider for MockNetworkProvider {
    async fn get_status(&self) -> Result<NetworkStatus, NetworkError> {
        Ok(self.status.lock().unwrap().clone())
    }

    async fn subscribe(&self) -> Result<NetworkStream, NetworkError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn set_wifi_enabled(&self, enabled: bool) -> Result<(), NetworkError> {
        let mut status = self.status.lock().unwrap();
        status.is_wifi_enabled = enabled;
        let _ = self.status_tx.send(status.clone());
        Ok(())
    }

    async fn scan_wifi(&self) -> Result<(), NetworkError> {
        Ok(())
    }

    async fn connect_to_ap(&self, id: &str, _password: Option<&str>) -> Result<(), NetworkError> {
        let mut status = self.status.lock().unwrap();
        let mut found = false;
        let mut strength = 0;

        for ap in status.access_points.iter_mut() {
            ap.is_active = ap.id == id;
            if ap.is_active {
                found = true;
                strength = ap.strength;
            }
        }

        if found {
            status.is_wifi_connected = true;
            status.active_strength = strength;
        }

        let _ = self.status_tx.send(status.clone());
        Ok(())
    }

    async fn disconnect_wifi(&self) -> Result<(), NetworkError> {
        let mut status = self.status.lock().unwrap();
        status.is_wifi_connected = false;
        for ap in status.access_points.iter_mut() {
            ap.is_active = false;
        }
        let _ = self.status_tx.send(status.clone());
        Ok(())
    }
}
