use axis_domain::models::calendar::CalendarEvent;
use axis_domain::models::tasks::{Task, TaskList};
use axis_domain::ports::agenda::{AgendaProvider, AgendaError};
use async_trait::async_trait;
use std::sync::Arc;

pub struct MockAgendaProvider;

impl MockAgendaProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl AgendaProvider for MockAgendaProvider {
    async fn fetch_events(
        &self,
        _start: &str,
        _end: &str,
    ) -> Result<Vec<CalendarEvent>, AgendaError> {
        Ok(Vec::new())
    }

    async fn fetch_lists(&self) -> Result<Vec<TaskList>, AgendaError> {
        Ok(Vec::new())
    }

    async fn fetch_tasks(&self, _list_id: &str) -> Result<Vec<Task>, AgendaError> {
        Ok(Vec::new())
    }

    async fn toggle_task(
        &self,
        _list_id: &str,
        _task_id: &str,
        _done: bool,
    ) -> Result<(), AgendaError> {
        Ok(())
    }

    async fn delete_task(&self, _list_id: &str, _task_id: &str) -> Result<(), AgendaError> {
        Ok(())
    }

    async fn create_task(
        &self,
        _list_id: &str,
        _title: &str,
    ) -> Result<Task, AgendaError> {
        Ok(Task {
            id: "mock-task-1".to_string(),
            title: _title.to_string(),
            done: false,
            list_id: _list_id.to_string(),
        })
    }
}
