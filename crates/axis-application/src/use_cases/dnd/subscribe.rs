use axis_domain::ports::dnd::{DndProvider, DndError, DndStream};
use std::sync::Arc;

pub struct SubscribeToDndUpdatesUseCase {
    provider: Arc<dyn DndProvider>,
}

impl SubscribeToDndUpdatesUseCase {
    pub fn new(provider: Arc<dyn DndProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<DndStream, DndError> {
        self.provider.subscribe().await
    }
}
