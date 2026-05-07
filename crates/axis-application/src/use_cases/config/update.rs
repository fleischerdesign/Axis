use axis_domain::models::config::AxisConfig;
use axis_domain::ports::config::{ConfigError, ConfigProvider};
use log::debug;
use std::sync::Arc;

pub struct UpdateConfigUseCase {
    provider: Arc<dyn ConfigProvider>,
}

impl UpdateConfigUseCase {
    pub fn new(provider: Arc<dyn ConfigProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute<F>(&self, update_fn: F) -> Result<(), ConfigError>
    where
        F: FnOnce(&mut AxisConfig) + Send + 'static,
    {
        debug!("[use-case] Updating global configuration");
        self.provider.update(Box::new(update_fn))
    }
}
