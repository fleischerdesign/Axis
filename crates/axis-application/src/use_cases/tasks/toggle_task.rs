use axis_domain::ports::tasks::{TaskProvider, TaskError};
use std::sync::Arc;
use log::{info, error};

pub struct ToggleTaskUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl ToggleTaskUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str, task_id: &str, done: bool) -> Result<(), TaskError> {
        info!("[use-case] Toggling task {} to done={}", task_id, done);

        match self.provider.toggle_task(list_id, task_id, done).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("[use-case] Failed to toggle task: {}", e);
                Err(e)
            }
        }
    }
}
