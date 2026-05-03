use axis_domain::ports::mpris::{MprisProvider, MprisError};
use std::sync::Arc;

pub struct PreviousTrackUseCase {
    provider: Arc<dyn MprisProvider>,
}

impl PreviousTrackUseCase {
    pub fn new(provider: Arc<dyn MprisProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, player_id: &str) -> Result<(), MprisError> {
        self.provider.previous(player_id).await
    }
}
