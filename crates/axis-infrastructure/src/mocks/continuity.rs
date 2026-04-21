use axis_domain::models::continuity::{Peer, PeerStatus, ContinuityMessage};
use axis_domain::ports::continuity::{PeerDiscovery, PeerConnection, ContinuityError};
use async_trait::async_trait;
use std::sync::Mutex;


pub struct MockContinuityProvider {
    peers: Mutex<Vec<Peer>>,
}

impl MockContinuityProvider {
    pub fn new() -> Self {
        Self {
            peers: Mutex::new(vec![
                Peer {
                    id: "peer-1".to_string(),
                    name: "Laptop-Alpha".to_string(),
                    address: "192.168.1.50:7391".parse().unwrap(),
                    status: PeerStatus::Disconnected,
                },
                Peer {
                    id: "peer-2".to_string(),
                    name: "Tablet-Beta".to_string(),
                    address: "192.168.1.60:7391".parse().unwrap(),
                    status: PeerStatus::Connected,
                },
            ]),
        }
    }
}

#[async_trait]
impl PeerDiscovery for MockContinuityProvider {
    async fn start_browsing(&self) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn stop_browsing(&self) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn get_discovered_peers(&self) -> Result<Vec<Peer>, ContinuityError> {
        Ok(self.peers.lock().unwrap().clone())
    }
}

#[async_trait]
impl PeerConnection for MockContinuityProvider {
    async fn connect(&self, _peer: &Peer) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), ContinuityError> {
        Ok(())
    }

    async fn send_message(&self, _msg: ContinuityMessage) -> Result<(), ContinuityError> {
        Ok(())
    }
}
