use axis_domain::models::tasks::Task;
use axis_domain::ports::tasks::{TaskProvider, TaskError};
use std::sync::Arc;

pub struct GetTasksUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl GetTasksUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str) -> Result<Vec<Task>, TaskError> {
        self.provider.get_tasks(list_id).await
    }
}
