use axis_domain::models::config::AxisConfig;
use axis_domain::ports::config::{ConfigProvider, ConfigError};
use std::sync::Arc;
use log::debug;

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
