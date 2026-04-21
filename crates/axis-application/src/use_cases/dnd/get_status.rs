use axis_domain::models::dnd::DndStatus;
use axis_domain::ports::dnd::{DndProvider, DndError};
use std::sync::Arc;

pub struct GetDndStatusUseCase {
    provider: Arc<dyn DndProvider>,
}

impl GetDndStatusUseCase {
    pub fn new(provider: Arc<dyn DndProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<DndStatus, DndError> {
        self.provider.get_status().await
    }
}
