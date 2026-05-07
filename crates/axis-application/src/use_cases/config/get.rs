use axis_domain::models::config::AxisConfig;
use axis_domain::ports::config::{ConfigError, ConfigProvider};
use log::debug;
use std::sync::Arc;

pub struct GetConfigUseCase {
    provider: Arc<dyn ConfigProvider>,
}

impl GetConfigUseCase {
    pub fn new(provider: Arc<dyn ConfigProvider>) -> Self {
        Self { provider }
    }

    pub fn execute(&self) -> Result<AxisConfig, ConfigError> {
        debug!("[use-case] Fetching config");
        self.provider.get()
    }
}
