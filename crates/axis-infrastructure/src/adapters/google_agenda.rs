use axis_domain::models::calendar::CalendarEvent;
use axis_domain::models::tasks::{Task, TaskList};
use axis_domain::ports::agenda::{AgendaProvider, AgendaError};
use async_trait::async_trait;
use std::sync::Arc;

pub struct GoogleAgendaProvider {
    calendar: Arc<dyn axis_domain::ports::calendar::CalendarProvider>,
    tasks: Arc<dyn axis_domain::ports::tasks::TaskProvider>,
}

impl GoogleAgendaProvider {
    pub fn new(
        calendar: Arc<dyn axis_domain::ports::calendar::CalendarProvider>,
        tasks: Arc<dyn axis_domain::ports::tasks::TaskProvider>,
    ) -> Arc<Self> {
        Arc::new(Self { calendar, tasks })
    }
}

#[async_trait]
impl AgendaProvider for GoogleAgendaProvider {
    async fn fetch_events(
        &self,
        start: &str,
        end: &str,
    ) -> Result<Vec<CalendarEvent>, AgendaError> {
        self.calendar
            .get_events(start, end)
            .await
            .map_err(|e| AgendaError::ProviderError(e.to_string()))
    }

    async fn fetch_lists(&self) -> Result<Vec<TaskList>, AgendaError> {
        self.tasks
            .get_lists()
            .await
            .map_err(|e| AgendaError::ProviderError(e.to_string()))
    }

    async fn fetch_tasks(&self, list_id: &str) -> Result<Vec<Task>, AgendaError> {
        self.tasks
            .get_tasks(list_id)
            .await
            .map_err(|e| AgendaError::ProviderError(e.to_string()))
    }

    async fn toggle_task(
        &self,
        list_id: &str,
        task_id: &str,
        done: bool,
    ) -> Result<(), AgendaError> {
        self.tasks
            .toggle_task(list_id, task_id, done)
            .await
            .map_err(|e| AgendaError::ProviderError(e.to_string()))
    }

    async fn delete_task(&self, list_id: &str, task_id: &str) -> Result<(), AgendaError> {
        self.tasks
            .delete_task(list_id, task_id)
            .await
            .map_err(|e| AgendaError::ProviderError(e.to_string()))
    }

    async fn create_task(&self, list_id: &str, title: &str) -> Result<Task, AgendaError> {
        self.tasks
            .create_task(list_id, title)
            .await
            .map_err(|e| AgendaError::ProviderError(e.to_string()))
    }
}
