use std::pin::Pin;
use std::time::Duration;

use async_channel::Sender;
use axis_domain::models::continuity::ReconnectState;
use log::{info, warn};

use super::super::clipboard::ClipboardEvent;
use super::super::connection::{ConnectionEvent, ConnectionProvider, TcpConnectionProvider};
use super::{ContinuityInner, RECONNECT_BASE_DELAY_MS, RECONNECT_MAX_ATTEMPTS};

impl ContinuityInner {
    pub(crate) async fn handle_clipboard_event(
        &mut self,
        event: ClipboardEvent,
        connection: &TcpConnectionProvider,
    ) {
        match event {
            ClipboardEvent::ContentChanged { content, mime_type } => {
                if self.status.active_connection.is_some() {
                    info!("[continuity] clipboard changed, sending to peer");
                    connection.send_message(
                        axis_domain::models::continuity::Message::ClipboardUpdate {
                            content,
                            mime_type,
                        },
                    );
                }
            }
        }
    }

    pub(crate) fn schedule_reconnect(&mut self) -> Option<Pin<Box<tokio::time::Sleep>>> {
        let reconnect = match &self.status.reconnect {
            Some(r) => r.clone(),
            None => return None,
        };

        if reconnect.attempt > reconnect.max_attempts {
            warn!(
                "[continuity] reconnect failed after {} attempts, giving up",
                reconnect.attempt - 1
            );
            self.status.reconnect = None;
            self.push();
            return None;
        }

        let delay_secs = reconnect.delay_secs;
        info!(
            "[continuity] scheduling reconnect for {} (attempt {}/{}, in {}s)",
            reconnect.peer_name, reconnect.attempt, reconnect.max_attempts, delay_secs
        );

        Some(Box::pin(tokio::time::sleep(Duration::from_secs(
            delay_secs,
        ))))
    }

    pub(crate) fn start_reconnect(
        &mut self,
        peer_id: &str,
        peer_name: &str,
    ) -> Option<Pin<Box<tokio::time::Sleep>>> {
        if self.status.reconnect.is_some() {
            return None;
        }
        let attempt = 1;
        let delay_secs = RECONNECT_BASE_DELAY_MS / 1000;
        self.status.reconnect = Some(ReconnectState {
            peer_id: peer_id.to_string(),
            peer_name: peer_name.to_string(),
            attempt,
            max_attempts: RECONNECT_MAX_ATTEMPTS,
            delay_secs,
        });
        self.schedule_reconnect()
    }

    pub(crate) fn handle_reconnect_attempt(
        &mut self,
        connection: &mut TcpConnectionProvider,
        conn_tx: &Sender<ConnectionEvent>,
    ) -> Option<Pin<Box<tokio::time::Sleep>>> {
        let reconnect = match &self.status.reconnect {
            Some(r) => r.clone(),
            None => return None,
        };

        info!(
            "[continuity] reconnect attempt {}/{} for {}",
            reconnect.attempt, reconnect.max_attempts, reconnect.peer_name
        );

        let peer_info = self
            .status
            .peers
            .iter()
            .find(|p| p.device_id == reconnect.peer_id)
            .cloned();

        if let Some(peer) = peer_info {
            self.is_initiating = true;
            self.pending_peer = Some((peer.device_id.clone(), peer.device_name.clone()));

            connection.connect_dual(
                peer.address,
                peer.address_v6,
                conn_tx.clone(),
                self.status.device_id.clone(),
                self.status.device_name.clone(),
            );

            let next_delay = RECONNECT_BASE_DELAY_MS * 2u64.pow(reconnect.attempt - 1) / 1000;
            if let Some(ref mut r) = self.status.reconnect {
                r.attempt += 1;
                r.delay_secs = next_delay;
            }
            self.push();

            self.schedule_reconnect()
        } else {
            if let Some(ref mut r) = self.status.reconnect {
                r.delay_secs = 5;
            }
            self.schedule_reconnect()
        }
    }
}
