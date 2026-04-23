use axis_domain::models::tasks::{Task, TaskList};
use axis_domain::ports::tasks::TaskProvider;
use std::sync::Arc;

pub struct SyncTasksUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl SyncTasksUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: Option<String>) -> Result<(Vec<TaskList>, Vec<Task>, Option<String>), String> {
        let task_lists = self.provider.get_lists().await
            .map_err(|e| e.to_string())?;

        let selected_id = list_id.or_else(|| task_lists.first().map(|l| l.id.clone()));
        
        let mut tasks = Vec::new();
        if let Some(ref id) = selected_id {
            tasks = self.provider.get_tasks(id).await
                .map_err(|e| e.to_string())?;
        }

        Ok((task_lists, tasks, selected_id))
    }
}
