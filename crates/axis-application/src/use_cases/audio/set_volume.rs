use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;

pub struct SetVolumeUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SetVolumeUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, volume: f64) -> Result<(), AudioError> {
        self.provider.set_volume(volume).await
    }
}
