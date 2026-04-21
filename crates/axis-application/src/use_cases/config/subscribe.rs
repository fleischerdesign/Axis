use axis_domain::ports::config::{ConfigProvider, ConfigStream};
use std::sync::Arc;

pub struct SubscribeToConfigUseCase {
    provider: Arc<dyn ConfigProvider>,
}

impl SubscribeToConfigUseCase {
    pub fn new(provider: Arc<dyn ConfigProvider>) -> Self {
        Self { provider }
    }

    pub fn execute(&self) -> ConfigStream {
        self.provider.subscribe()
    }
}
