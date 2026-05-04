use axis_domain::ports::agenda::{AgendaProvider, AgendaError};
use std::sync::Arc;
use log::info;

pub struct DeleteTaskUseCase {
    provider: Arc<dyn AgendaProvider>,
}

impl DeleteTaskUseCase {
    pub fn new(provider: Arc<dyn AgendaProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str, task_id: &str) -> Result<(), AgendaError> {
        info!("[use-case] Deleting task: {}", task_id);
        self.provider.delete_task(list_id, task_id).await
    }
}
