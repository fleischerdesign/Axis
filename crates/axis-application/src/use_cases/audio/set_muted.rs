use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;
use log::info;

pub struct SetMutedUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SetMutedUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, muted: bool) -> Result<(), AudioError> {
        info!("[use-case] Setting system mute to: {}", muted);
        self.provider.set_muted(muted).await
    }
}
