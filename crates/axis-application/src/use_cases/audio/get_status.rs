use axis_domain::models::audio::AudioStatus;
use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;

pub struct GetAudioStatusUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl GetAudioStatusUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<AudioStatus, AudioError> {
        self.provider.get_status().await
    }
}
