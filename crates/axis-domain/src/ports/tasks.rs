use crate::models::tasks::{Task, TaskList};
use crate::models::cloud::AuthStatus;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum TaskError {
    #[error("Task provider error: {0}")]
    ProviderError(String),
}

#[async_trait]
pub trait TaskProvider: Send + Sync {
    async fn get_lists(&self) -> Result<Vec<TaskList>, TaskError>;
    async fn get_tasks(&self, list_id: &str) -> Result<Vec<Task>, TaskError>;
    async fn toggle_task(&self, list_id: &str, task_id: &str, done: bool) -> Result<(), TaskError>;
    async fn delete_task(&self, list_id: &str, task_id: &str) -> Result<(), TaskError>;
    async fn create_task(&self, list_id: &str, title: &str) -> Result<Task, TaskError>;
    async fn get_auth_status(&self) -> Result<AuthStatus, TaskError>;
}
