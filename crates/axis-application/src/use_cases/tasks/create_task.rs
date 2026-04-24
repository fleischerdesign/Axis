use axis_domain::models::tasks::Task;
use axis_domain::ports::tasks::{TaskProvider, TaskError};
use std::sync::Arc;
use log::{info, error};

pub struct CreateTaskUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl CreateTaskUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str, title: &str) -> Result<Task, TaskError> {
        if title.trim().is_empty() {
            return Err(TaskError::ProviderError("Task title cannot be empty".to_string()));
        }

        info!("[use-case] Creating new task: {}", title);

        match self.provider.create_task(list_id, title).await {
            Ok(task) => {
                info!("[use-case] Task created successfully with ID: {}", task.id);
                Ok(task)
            }
            Err(e) => {
                error!("[use-case] Failed to create task: {}", e);
                Err(e)
            }
        }
    }
}
