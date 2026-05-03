use axis_domain::ports::mpris::{MprisProvider, MprisError};
use std::sync::Arc;

pub struct PlayPauseUseCase {
    provider: Arc<dyn MprisProvider>,
}

impl PlayPauseUseCase {
    pub fn new(provider: Arc<dyn MprisProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, player_id: &str) -> Result<(), MprisError> {
        self.provider.play_pause(player_id).await
    }
}
