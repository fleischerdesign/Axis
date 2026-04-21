use axis_domain::ports::audio::{AudioProvider, AudioError, AudioStream};
use std::sync::Arc;

pub struct SubscribeToAudioUpdatesUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SubscribeToAudioUpdatesUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<AudioStream, AudioError> {
        self.provider.subscribe().await
    }
}
