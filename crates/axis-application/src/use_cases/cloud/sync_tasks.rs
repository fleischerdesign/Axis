use axis_domain::models::tasks::{Task, TaskList};
use axis_domain::ports::tasks::TaskProvider;
use std::sync::Arc;
use log::{info, error};

pub struct SyncTasksUseCase {
    provider: Arc<dyn TaskProvider>,
}

impl SyncTasksUseCase {
    pub fn new(provider: Arc<dyn TaskProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, list_id: Option<String>) -> Result<(Vec<TaskList>, Vec<Task>, Option<String>), String> {
        info!("[use-case] Syncing task lists and tasks");

        let task_lists = match self.provider.get_lists().await {
            Ok(lists) => lists,
            Err(e) => {
                let err_msg = format!("Failed to fetch task lists: {}", e);
                error!("[use-case] {}", err_msg);
                return Err(err_msg);
            }
        };

        let selected_id = list_id.or_else(|| task_lists.first().map(|l| l.id.clone()));
        
        let mut tasks = Vec::new();
        if let Some(ref id) = selected_id {
            tasks = match self.provider.get_tasks(id).await {
                Ok(t) => t,
                Err(e) => {
                    let err_msg = format!("Failed to fetch tasks for list {}: {}", id, e);
                    error!("[use-case] {}", err_msg);
                    return Err(err_msg);
                }
            };
        }

        info!("[use-case] Task sync complete ({} lists, {} tasks)", task_lists.len(), tasks.len());
        Ok((task_lists, tasks, selected_id))
    }
}
