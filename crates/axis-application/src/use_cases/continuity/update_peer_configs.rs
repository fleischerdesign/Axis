use axis_domain::models::continuity::PeerConfig;
use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::collections::HashMap;
use std::sync::Arc;
use log::info;

pub struct UpdatePeerConfigsUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl UpdatePeerConfigsUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(
        &self,
        configs: HashMap<String, PeerConfig>,
    ) -> Result<(), ContinuityError> {
        info!("[use-case] Updating peer configs");
        self.provider.update_peer_configs(configs).await
    }
}
