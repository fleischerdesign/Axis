use axis_domain::models::config::AxisConfig;
use axis_domain::ports::config::ConfigProvider;
use std::sync::Arc;

pub struct GetConfigUseCase {
    provider: Arc<dyn ConfigProvider>,
}

impl GetConfigUseCase {
    pub fn new(provider: Arc<dyn ConfigProvider>) -> Self {
        Self { provider }
    }

    pub fn execute(&self) -> AxisConfig {
        self.provider.get()
    }
}
