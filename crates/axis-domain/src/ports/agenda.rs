use crate::models::calendar::CalendarEvent;
use crate::models::tasks::{Task, TaskList};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum AgendaError {
    #[error("Agenda provider error: {0}")]
    ProviderError(String),
}

#[async_trait]
pub trait AgendaProvider: Send + Sync {
    async fn fetch_events(&self, start: &str, end: &str)
        -> Result<Vec<CalendarEvent>, AgendaError>;
    async fn fetch_lists(&self) -> Result<Vec<TaskList>, AgendaError>;
    async fn fetch_tasks(&self, list_id: &str) -> Result<Vec<Task>, AgendaError>;
    async fn toggle_task(
        &self,
        list_id: &str,
        task_id: &str,
        done: bool,
    ) -> Result<(), AgendaError>;
    async fn delete_task(&self, list_id: &str, task_id: &str) -> Result<(), AgendaError>;
    async fn create_task(
        &self,
        list_id: &str,
        title: &str,
    ) -> Result<Task, AgendaError>;
}
