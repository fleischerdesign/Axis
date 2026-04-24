use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;
use log::debug;

pub struct SetSinkInputVolumeUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SetSinkInputVolumeUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: u32, volume: f64) -> Result<(), AudioError> {
        let volume = volume.clamp(0.0, 1.5);
        debug!("[use-case] Setting sink input {} volume to {:.0}%", id, volume * 100.0);
        self.provider.set_sink_input_volume(id, volume).await
    }
}
