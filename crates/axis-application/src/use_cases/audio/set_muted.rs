use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;

pub struct SetMutedUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SetMutedUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, muted: bool) -> Result<(), AudioError> {
        self.provider.set_muted(muted).await
    }
}
