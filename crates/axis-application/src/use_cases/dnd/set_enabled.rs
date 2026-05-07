use axis_domain::ports::dnd::{DndError, DndProvider};
use log::debug;
use std::sync::Arc;

pub struct SetDndEnabledUseCase {
    provider: Arc<dyn DndProvider>,
}

impl SetDndEnabledUseCase {
    pub fn new(provider: Arc<dyn DndProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), DndError> {
        debug!(
            "[use-case] Setting Do-Not-Disturb (DND) mode to: {}",
            enabled
        );
        self.provider.set_enabled(enabled).await
    }
}
