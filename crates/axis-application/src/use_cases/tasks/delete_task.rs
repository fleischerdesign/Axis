use axis_domain::ports::tasks::{TaskProvider, TaskError};
use std::sync::Arc;
use log::{info, error};

pub struct DeleteTaskUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl DeleteTaskUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str, task_id: &str) -> Result<(), TaskError> {
        info!("[use-case] Deleting task: {}", task_id);

        match self.provider.delete_task(list_id, task_id).await {
            Ok(_) => {
                info!("[use-case] Task deleted successfully");
                Ok(())
            }
            Err(e) => {
                error!("[use-case] Failed to delete task: {}", e);
                Err(e)
            }
        }
    }
}
