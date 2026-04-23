use axis_domain::models::tasks::Task;
use axis_domain::ports::tasks::{TaskProvider, TaskError};
use std::sync::Arc;

pub struct CreateTaskUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl CreateTaskUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str, title: &str) -> Result<Task, TaskError> {
        self.provider.create_task(list_id, title).await
    }
}
