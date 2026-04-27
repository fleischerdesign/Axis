use axis_domain::models::config::AxisConfig;
use axis_domain::ports::config::{ConfigError, ConfigProvider};
use std::sync::Arc;

pub struct GetConfigUseCase {
    provider: Arc<dyn ConfigProvider>,
}

impl GetConfigUseCase {
    pub fn new(provider: Arc<dyn ConfigProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<AxisConfig, ConfigError> {
        self.provider.get()
    }
}
