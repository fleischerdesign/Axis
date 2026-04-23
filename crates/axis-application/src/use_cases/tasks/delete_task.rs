use axis_domain::ports::tasks::{TaskProvider, TaskError};
use std::sync::Arc;

pub struct DeleteTaskUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl DeleteTaskUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str, task_id: &str) -> Result<(), TaskError> {
        self.provider.delete_task(list_id, task_id).await
    }
}
