use axis_domain::models::calendar::CalendarEvent;
use axis_domain::models::tasks::{Task, TaskList};
use axis_domain::ports::agenda::{AgendaProvider, AgendaError};
use std::sync::Arc;
use chrono::{Utc, Duration};
use log::{info, error};

pub struct AgendaUseCase {
    provider: Arc<dyn AgendaProvider>,
}

impl AgendaUseCase {
    pub fn new(provider: Arc<dyn AgendaProvider>) -> Self {
        Self { provider }
    }

    pub async fn sync_events(&self) -> Result<Vec<CalendarEvent>, AgendaError> {
        let now = Utc::now();
        let start = (now - Duration::days(30)).to_rfc3339();
        let end = (now + Duration::days(60)).to_rfc3339();

        info!("[use-case] Syncing calendar events");
        self.provider.fetch_events(&start, &end).await
    }

    pub async fn sync_tasks(
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

    pub async fn toggle_task(
        &self,
        list_id: &str,
        task_id: &str,
        done: bool,
    ) -> Result<(), AgendaError> {
        info!("[use-case] Toggling task {} to done={}", task_id, done);
        self.provider.toggle_task(list_id, task_id, done).await
    }

    pub async fn delete_task(
        &self,
        list_id: &str,
        task_id: &str,
    ) -> Result<(), AgendaError> {
        info!("[use-case] Deleting task: {}", task_id);
        self.provider.delete_task(list_id, task_id).await
    }

    pub async fn create_task(
        &self,
        list_id: &str,
        title: &str,
    ) -> Result<Task, AgendaError> {
        if title.trim().is_empty() {
            error!("[use-case] Task title cannot be empty");
            return Err(AgendaError::ProviderError(
                "Task title cannot be empty".to_string(),
            ));
        }
        info!("[use-case] Creating new task: {}", title);
        self.provider.create_task(list_id, title).await
    }
}
