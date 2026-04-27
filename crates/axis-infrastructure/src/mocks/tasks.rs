use axis_domain::models::tasks::{Task, TaskList};
use axis_domain::models::cloud::AuthStatus;
use axis_domain::ports::tasks::{TaskProvider, TaskError};
use async_trait::async_trait;

pub struct MockTaskProvider;

#[async_trait]
impl TaskProvider for MockTaskProvider {
    async fn get_lists(&self) -> Result<Vec<TaskList>, TaskError> {
        Ok(vec![TaskList { id: "default".to_string(), title: "Personal".to_string() }])
    }
    async fn get_tasks(&self, _list_id: &str) -> Result<Vec<Task>, TaskError> {
        Ok(vec![
            Task { id: "1".to_string(), title: "Rewrite Axis".to_string(), done: false, list_id: "default".to_string() },
            Task { id: "2".to_string(), title: "Clean Code".to_string(), done: true, list_id: "default".to_string() },
        ])
    }
    async fn toggle_task(&self, _l: &str, _t: &str, _d: bool) -> Result<(), TaskError> { Ok(()) }
    async fn delete_task(&self, _l: &str, _t: &str) -> Result<(), TaskError> { Ok(()) }
    async fn create_task(&self, list_id: &str, title: &str) -> Result<Task, TaskError> {
        Ok(Task {
            id: "new".to_string(),
            title: title.to_string(),
            done: false,
            list_id: list_id.to_string(),
        })
    }
    async fn get_auth_status(&self) -> Result<AuthStatus, TaskError> { Ok(AuthStatus::Authenticated) }
}
