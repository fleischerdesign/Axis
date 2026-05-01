use axis_domain::models::continuity::PeerConfig;
use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::collections::HashMap;
use std::sync::Arc;

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
        self.provider.update_peer_configs(configs).await
    }
}
