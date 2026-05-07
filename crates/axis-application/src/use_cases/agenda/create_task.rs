use axis_domain::models::tasks::Task;
use axis_domain::ports::agenda::{AgendaError, AgendaProvider};
use log::{error, info};
use std::sync::Arc;

pub struct CreateTaskUseCase {
    provider: Arc<dyn AgendaProvider>,
}

impl CreateTaskUseCase {
    pub fn new(provider: Arc<dyn AgendaProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str, title: &str) -> Result<Task, AgendaError> {
        if title.trim().is_empty() {
            error!("[use-case] Task title cannot be empty");
            return Err(AgendaError::ValidationError(
                "Task title cannot be empty".to_string(),
            ));
        }
        info!("[use-case] Creating new task: {}", title);
        self.provider.create_task(list_id, title).await
    }
}
