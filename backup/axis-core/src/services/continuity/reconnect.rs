use async_channel::Sender;
use log::{info, warn};
use std::pin::Pin;
use std::time::Duration;

use super::{
    ContinuityInner, RECONNECT_MAX_ATTEMPTS, RECONNECT_BASE_DELAY_MS,
};
use super::connection::{ConnectionEvent, ConnectionProvider, TcpConnectionProvider};

pub(super) type ReconnectSleep = Pin<Box<tokio::time::Sleep>>;

impl ContinuityInner {
    pub(super) fn schedule_reconnect(&mut self) -> Option<ReconnectSleep> {
        let reconnect = match &self.data.reconnect {
            Some(r) => r.clone(),
            None => return None,
        };

        if reconnect.attempt > reconnect.max_attempts {
            warn!("[continuity] reconnect failed after {} attempts, giving up", reconnect.attempt - 1);
            self.data.reconnect = None;
            self.push();
            return None;
        }

        let delay_secs = reconnect.delay_secs;
        info!("[continuity] scheduling reconnect for {} (attempt {}/{}, in {}s)",
            reconnect.peer_name, reconnect.attempt, reconnect.max_attempts, delay_secs);

        Some(Box::pin(tokio::time::sleep(Duration::from_secs(delay_secs))))
    }

    pub(super) fn cancel_reconnect(&mut self) {
        self.data.reconnect = None;
    }

    pub(super) fn start_reconnect(&mut self, peer_id: &str, peer_name: &str) -> Option<ReconnectSleep> {
        if self.data.reconnect.is_some() {
            return None;
        }
        let attempt = 1;
        let delay_secs = RECONNECT_BASE_DELAY_MS / 1000;
        self.data.reconnect = Some(super::ReconnectState {
            peer_id: peer_id.to_string(),
            peer_name: peer_name.to_string(),
            attempt,
            max_attempts: RECONNECT_MAX_ATTEMPTS,
            delay_secs,
        });
        self.schedule_reconnect()
    }

    pub(super) fn handle_reconnect_attempt(
        &mut self,
        connection: &mut TcpConnectionProvider,
        conn_tx: &Sender<ConnectionEvent>,
    ) -> Option<ReconnectSleep> {
        let reconnect = match &self.data.reconnect {
            Some(r) => r.clone(),
            None => return None,
        };

        info!("[continuity] reconnect attempt {}/{} for {}",
            reconnect.attempt, reconnect.max_attempts, reconnect.peer_name);

        let peer_info = self.data.peers.iter()
            .find(|p| p.device_id == reconnect.peer_id)
            .cloned();

        if let Some(peer) = peer_info {
            self.is_initiating = true;
            self.pending_peer = Some((peer.device_id.clone(), peer.device_name.clone()));

            connection.connect_dual(
                peer.address,
                peer.address_v6,
                conn_tx.clone(),
                self.data.device_id.clone(),
                self.data.device_name.clone(),
            );

            let next_delay = RECONNECT_BASE_DELAY_MS * 2u64.pow(reconnect.attempt - 1) / 1000;
            self.data.reconnect.as_mut().unwrap().attempt += 1;
            self.data.reconnect.as_mut().unwrap().delay_secs = next_delay;
            self.push();

            self.schedule_reconnect()
        } else {
            self.data.reconnect.as_mut().unwrap().delay_secs = 5;
            self.schedule_reconnect()
        }
    }
}
