use axis_domain::models::continuity::{
    ContinuityStatus, InputEvent, PeerArrangement,
    PeerConfig, PeerInfo, SharingState, Side,
};
use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider, ContinuityStream};
use async_trait::async_trait;
use futures_util::stream;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct MockContinuityProvider {
    status: Mutex<ContinuityStatus>,
}

impl MockContinuityProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            status: Mutex::new(ContinuityStatus {
                device_id: "mock-device".to_string(),
                device_name: "Mock-Device".to_string(),
                enabled: true,
                peers: vec![
                    PeerInfo {
                        device_id: "peer-1".to_string(),
                        device_name: "Laptop-Alpha".to_string(),
                        hostname: "laptop-alpha.local".to_string(),
                        address: "192.168.1.50:7391".parse().unwrap(),
                        address_v6: None,
                    },
                    PeerInfo {
                        device_id: "peer-2".to_string(),
                        device_name: "Tablet-Beta".to_string(),
                        hostname: "tablet-beta.local".to_string(),
                        address: "192.168.1.60:7391".parse().unwrap(),
                        address_v6: None,
                    },
                ],
                sharing_state: SharingState::Idle,
                ..Default::default()
            }),
        })
    }
}

#[async_trait]
impl ContinuityProvider for MockContinuityProvider {
    async fn get_status(&self) -> Result<ContinuityStatus, ContinuityError> {
        Ok(self.status.lock().unwrap().clone())
    }

    async fn subscribe(&self) -> Result<ContinuityStream, ContinuityError> {
        let status = self.status.lock().unwrap().clone();
        Ok(Box::pin(stream::once(async move { status })))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), ContinuityError> {
        self.status.lock().unwrap().enabled = enabled;
        Ok(())
    }

    async fn connect_to_peer(&self, _peer_id: &str) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn confirm_pin(&self) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn reject_pin(&self) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn cancel_reconnect(&self) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn unpair(&self, _peer_id: &str) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn start_sharing(&self, _side: Side, _edge_pos: f64) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn stop_sharing(&self, _edge_pos: f64) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn send_input(&self, _event: InputEvent) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn force_local(&self) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn set_peer_arrangement(&self, _arrangement: PeerArrangement) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn update_peer_configs(&self, _configs: HashMap<String, PeerConfig>) -> Result<(), ContinuityError> {
        Ok(())
    }
}
