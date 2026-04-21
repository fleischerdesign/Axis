use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;

pub struct SetSinkInputVolumeUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SetSinkInputVolumeUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: u32, volume: f64) -> Result<(), AudioError> {
        self.provider.set_sink_input_volume(id, volume).await
    }
}
