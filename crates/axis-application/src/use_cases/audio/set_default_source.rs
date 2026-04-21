use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;

pub struct SetDefaultSourceUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SetDefaultSourceUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: u32) -> Result<(), AudioError> {
        self.provider.set_default_source(id).await
    }
}
