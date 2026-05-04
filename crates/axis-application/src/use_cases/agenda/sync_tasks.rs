use axis_domain::models::tasks::{Task, TaskList};
use axis_domain::ports::agenda::{AgendaProvider, AgendaError};
use std::sync::Arc;
use log::info;

pub struct SyncTasksUseCase {
    provider: Arc<dyn AgendaProvider>,
}

impl SyncTasksUseCase {
    pub fn new(provider: Arc<dyn AgendaProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(
        &self,
        list_id: Option<String>,
    ) -> Result<(Vec<TaskList>, Vec<Task>, Option<String>), AgendaError> {
        info!("[use-case] Syncing task lists and tasks");

        let lists = self.provider.fetch_lists().await?;
        let selected_id = list_id.or_else(|| lists.first().map(|l| l.id.clone()));

        let mut tasks = Vec::new();
        if let Some(ref id) = selected_id {
            tasks = self.provider.fetch_tasks(id).await?;
        }

        info!(
            "[use-case] Task sync complete ({} lists, {} tasks)",
            lists.len(),
            tasks.len()
        );
        Ok((lists, tasks, selected_id))
    }
}
