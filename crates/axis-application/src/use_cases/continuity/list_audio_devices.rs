use axis_domain::models::continuity::AudioDeviceInfo;
use axis_domain::ports::continuity::{ContinuityError, ContinuityProvider};
use std::sync::Arc;

pub struct ListAudioDevicesUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl ListAudioDevicesUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<Vec<AudioDeviceInfo>, ContinuityError> {
        self.provider.list_audio_devices().await
    }
}
