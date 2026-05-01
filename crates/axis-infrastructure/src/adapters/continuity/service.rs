use axis_domain::models::continuity::{
    ContinuityStatus, InputEvent, PeerArrangement, PeerConfig, Side,
};
use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider, ContinuityStream};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

use super::inner::{ContinuityCmd, ContinuityInner};

pub struct ContinuityService {
    cmd_tx: async_channel::Sender<ContinuityCmd>,
    status_tx: watch::Sender<ContinuityStatus>,
}

impl ContinuityService {
    pub fn new() -> Arc<Self> {
        let (status_tx, _) = watch::channel(ContinuityStatus::default());
        let status_tx_c = status_tx.clone();
        let (cmd_tx, cmd_rx) = async_channel::bounded(32);

        let svc = Arc::new(Self { cmd_tx, status_tx });

        tokio::spawn(async move {
            let mut inner = ContinuityInner::new(status_tx_c);
            inner.run(cmd_rx).await;
        });

        svc
    }
}

#[async_trait]
impl ContinuityProvider for ContinuityService {
    async fn get_status(&self) -> Result<ContinuityStatus, ContinuityError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<ContinuityStream, ContinuityError> {
        Ok(Box::pin(WatchStream::new(self.status_tx.subscribe())))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::SetEnabled(enabled))
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn connect_to_peer(&self, peer_id: &str) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::ConnectToPeer(peer_id.to_string()))
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn confirm_pin(&self) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::ConfirmPin)
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn reject_pin(&self) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::RejectPin)
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn disconnect(&self) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::Disconnect)
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn cancel_reconnect(&self) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::CancelReconnect)
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn unpair(&self, peer_id: &str) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::Unpair(peer_id.to_string()))
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn start_sharing(&self, side: Side, edge_pos: f64) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::StartSharing(side, edge_pos))
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn stop_sharing(&self, edge_pos: f64) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::StopSharing(edge_pos))
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn send_input(&self, event: InputEvent) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::SendInput(event))
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn force_local(&self) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::ForceLocal)
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn set_peer_arrangement(&self, arrangement: PeerArrangement) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::SetPeerArrangement(arrangement))
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }

    async fn update_peer_configs(
        &self,
        configs: HashMap<String, PeerConfig>,
    ) -> Result<(), ContinuityError> {
        self.cmd_tx
            .try_send(ContinuityCmd::UpdatePeerConfigs(configs))
            .map_err(|e| ContinuityError::ProviderError(e.to_string()))
    }
}
