use axis_domain::ports::tasks::{TaskProvider, TaskError};
use std::sync::Arc;

pub struct ToggleTaskUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl ToggleTaskUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: &str, task_id: &str, done: bool) -> Result<(), TaskError> {
        self.provider.toggle_task(list_id, task_id, done).await
    }
}
