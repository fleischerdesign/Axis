use axis_domain::models::tasks::{Task, TaskList};
use axis_domain::ports::tasks::{TaskProvider, TaskError};
use axis_domain::ports::cloud_auth::CloudAuthProvider;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

const GOOGLE_TASKS_API_URL: &str = "https://www.googleapis.com/tasks/v1";

#[derive(Deserialize)]
struct GoogleTaskListResponse {
    items: Option<Vec<GoogleTaskListItem>>,
}

#[derive(Deserialize)]
struct GoogleTaskListItem {
    id: String,
    title: String,
}

#[derive(Deserialize)]
struct GoogleTaskResponse {
    items: Option<Vec<GoogleTaskItem>>,
}

#[derive(Deserialize)]
struct GoogleTaskItem {
    id: String,
    title: String,
    status: String,
}

pub struct GoogleTasksAdapter {
    auth_provider: Arc<dyn CloudAuthProvider>,
    http_client: reqwest::Client,
}

impl GoogleTasksAdapter {
    pub fn new(auth_provider: Arc<dyn CloudAuthProvider>) -> Self {
        Self { 
            auth_provider,
            http_client: reqwest::Client::builder()
                .tcp_keepalive(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl TaskProvider for GoogleTasksAdapter {
    async fn get_lists(&self) -> Result<Vec<TaskList>, TaskError> {
        let scopes = vec!["https://www.googleapis.com/auth/tasks".to_string()];
        let token = self.auth_provider.get_token(&scopes).await
            .map_err(|e| TaskError::ProviderError(format!("Auth error: {}", e)))?;

        let resp = self.http_client.get(format!("{}/users/@me/lists", GOOGLE_TASKS_API_URL))
            .bearer_auth(&token)
            .send().await
            .map_err(|e| TaskError::ProviderError(e.to_string()))?;

        let list_resp: GoogleTaskListResponse = resp.json().await
            .map_err(|e| TaskError::ProviderError(e.to_string()))?;

        let lists: Vec<TaskList> = list_resp.items.unwrap_or_default().into_iter().map(|l| TaskList {
            id: l.id,
            title: l.title,
        }).collect();

        log::debug!("[google-tasks] Fetched {} task lists", lists.len());
        Ok(lists)
    }

    async fn get_tasks(&self, list_id: &str) -> Result<Vec<Task>, TaskError> {
        let scopes = vec!["https://www.googleapis.com/auth/tasks".to_string()];
        let token = self.auth_provider.get_token(&scopes).await
            .map_err(|e| TaskError::ProviderError(format!("Auth error: {}", e)))?;

        let resp = self.http_client.get(format!("{}/lists/{}/tasks", GOOGLE_TASKS_API_URL, list_id))
            .bearer_auth(&token)
            .send().await
            .map_err(|e| TaskError::ProviderError(e.to_string()))?;

        let task_resp: GoogleTaskResponse = resp.json().await
            .map_err(|e| TaskError::ProviderError(e.to_string()))?;

        let tasks: Vec<Task> = task_resp.items.unwrap_or_default().into_iter().map(|t| Task {
            id: t.id,
            title: t.title,
            done: t.status == "completed",
            list_id: list_id.to_string(),
        }).collect();

        log::debug!("[google-tasks] Fetched {} tasks from list {}", tasks.len(), list_id);
        Ok(tasks)
    }

    async fn toggle_task(&self, list_id: &str, task_id: &str, done: bool) -> Result<(), TaskError> {
        let scopes = vec!["https://www.googleapis.com/auth/tasks".to_string()];
        let token = self.auth_provider.get_token(&scopes).await
            .map_err(|e| TaskError::ProviderError(format!("Auth error: {}", e)))?;

        let status = if done { "completed" } else { "needsAction" };
        
        let resp = self.http_client.patch(format!("{}/lists/{}/tasks/{}", GOOGLE_TASKS_API_URL, list_id, task_id))
            .bearer_auth(&token)
            .json(&serde_json::json!({ "status": status }))
            .send().await
            .map_err(|e| TaskError::ProviderError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(TaskError::ProviderError(format!("API error: {}", resp.status())));
        }

        log::debug!("[google-tasks] Toggled task {} in list {} to done={}", task_id, list_id, done);
        Ok(())
    }

    async fn delete_task(&self, list_id: &str, task_id: &str) -> Result<(), TaskError> {
        let scopes = vec!["https://www.googleapis.com/auth/tasks".to_string()];
        let token = self.auth_provider.get_token(&scopes).await
            .map_err(|e| TaskError::ProviderError(format!("Auth error: {}", e)))?;

        let resp = self.http_client.delete(format!("{}/lists/{}/tasks/{}", GOOGLE_TASKS_API_URL, list_id, task_id))
            .bearer_auth(&token)
            .send().await
            .map_err(|e| TaskError::ProviderError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(TaskError::ProviderError(format!("API error: {}", resp.status())));
        }

        log::debug!("[google-tasks] Deleted task {} from list {}", task_id, list_id);
        Ok(())
    }

    async fn create_task(&self, list_id: &str, title: &str) -> Result<Task, TaskError> {
        let scopes = vec!["https://www.googleapis.com/auth/tasks".to_string()];
        let token = self.auth_provider.get_token(&scopes).await
            .map_err(|e| TaskError::ProviderError(format!("Auth error: {}", e)))?;

        let resp = self.http_client.post(format!("{}/lists/{}/tasks", GOOGLE_TASKS_API_URL, list_id))
            .bearer_auth(&token)
            .json(&serde_json::json!({ "title": title }))
            .send().await
            .map_err(|e| TaskError::ProviderError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(TaskError::ProviderError(format!("API error: {}", resp.status())));
        }

        let google_task: GoogleTaskItem = resp.json().await
            .map_err(|e| TaskError::ProviderError(e.to_string()))?;

        log::debug!("[google-tasks] Created task '{}' in list {}", title, list_id);
        
        Ok(Task {
            id: google_task.id,
            title: google_task.title,
            done: google_task.status == "completed",
            list_id: list_id.to_string(),
        })
    }

    async fn get_auth_status(&self) -> Result<axis_domain::models::tasks::AuthStatus, TaskError> {
        if self.auth_provider.is_authenticated().await {
            Ok(axis_domain::models::tasks::AuthStatus::Authenticated)
        } else {
            Ok(axis_domain::models::tasks::AuthStatus::Failed("Not authenticated".into()))
        }
    }
}
