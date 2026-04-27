use axis_domain::ports::config::{ConfigError, ConfigProvider, ConfigStream};
use std::sync::Arc;

pub struct SubscribeToConfigUseCase {
    provider: Arc<dyn ConfigProvider>,
}

impl SubscribeToConfigUseCase {
    pub fn new(provider: Arc<dyn ConfigProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<ConfigStream, ConfigError> {
        self.provider.subscribe()
    }
}
