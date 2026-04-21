use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;

pub struct SetDefaultSinkUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SetDefaultSinkUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: u32) -> Result<(), AudioError> {
        self.provider.set_default_sink(id).await
    }
}
