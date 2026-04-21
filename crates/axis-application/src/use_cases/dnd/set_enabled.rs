use axis_domain::ports::dnd::{DndProvider, DndError};
use std::sync::Arc;

pub struct SetDndEnabledUseCase {
    provider: Arc<dyn DndProvider>,
}

impl SetDndEnabledUseCase {
    pub fn new(provider: Arc<dyn DndProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, enabled: bool) -> Result<(), DndError> {
        self.provider.set_enabled(enabled).await
    }
}
