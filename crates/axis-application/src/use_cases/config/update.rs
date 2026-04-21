use axis_domain::models::config::AxisConfig;
use axis_domain::ports::config::ConfigProvider;
use std::sync::Arc;

pub struct UpdateConfigUseCase {
    provider: Arc<dyn ConfigProvider>,
}

impl UpdateConfigUseCase {
    pub fn new(provider: Arc<dyn ConfigProvider>) -> Self {
        Self { provider }
    }

    pub fn execute(&self, apply: Box<dyn FnOnce(&mut AxisConfig) + Send + 'static>) {
        self.provider.update(apply);
    }
}
