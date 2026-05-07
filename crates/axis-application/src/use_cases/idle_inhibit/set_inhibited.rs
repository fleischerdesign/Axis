use axis_domain::ports::idle_inhibit::{IdleInhibitError, IdleInhibitProvider};
use log::info;
use std::sync::Arc;

pub struct SetIdleInhibitUseCase {
    provider: Arc<dyn IdleInhibitProvider>,
}

impl SetIdleInhibitUseCase {
    pub fn new(provider: Arc<dyn IdleInhibitProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, inhibited: bool) -> Result<(), IdleInhibitError> {
        info!("[use-case] Setting idle inhibit to: {}", inhibited);
        self.provider.set_inhibited(inhibited).await
    }
}
