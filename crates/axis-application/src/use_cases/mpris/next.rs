use axis_domain::ports::mpris::{MprisProvider, MprisError};
use std::sync::Arc;

pub struct NextTrackUseCase {
    provider: Arc<dyn MprisProvider>,
}

impl NextTrackUseCase {
    pub fn new(provider: Arc<dyn MprisProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, player_id: &str) -> Result<(), MprisError> {
        self.provider.next(player_id).await
    }
}
