use super::provider::{AuthStatus, Task, TaskList, TaskProvider};
use crate::services::google::GoogleAuthRegistry;
use crate::services::tasks::utils::{api_get, api_patch, api_post, build_http_client};
use log::info;
use serde::{Deserialize, Serialize};

const TASKS_SCOPE: &[&str] = &["https://www.googleapis.com/auth/tasks"];

pub struct GoogleTasksProvider {
    http_client: reqwest::blocking::Client,
}

impl GoogleTasksProvider {
    pub fn new() -> Self {
        Self {
            http_client: build_http_client().unwrap_or_else(|e| {
                log::warn!("[google-tasks] Failed to build HTTP client: {e}");
                reqwest::blocking::Client::new()
            }),
        }
    }
}

impl Default for GoogleTasksProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskProvider for GoogleTasksProvider {
    fn name(&self) -> &str {
        "Google Tasks"
    }

    fn icon(&self) -> &str {
        "google-symbolic"
    }

    fn auth_status(&mut self) -> AuthStatus {
        crate::services::google::google_auth_status()
    }

    fn authenticate(&mut self) -> Result<AuthStatus, String> {
        crate::services::google::google_authenticate(TASKS_SCOPE);
        Ok(AuthStatus::Authenticated)
    }

    fn is_authenticated(&self) -> bool {
        crate::services::google::google_is_authenticated()
    }

    fn lists(&mut self) -> Result<Vec<TaskList>, String> {
        let mut reg = GoogleAuthRegistry::load()?;
        let token = reg.ensure_token(TASKS_SCOPE)?;

        #[derive(Deserialize)]
        struct Response {
            items: Option<Vec<Item>>,
        }

        #[derive(Deserialize)]
        struct Item {
            id: String,
            title: String,
        }

        let resp: Response = api_get(&self.http_client, "https://tasks.googleapis.com/tasks/v1/users/@me/lists", &token)?;

        Ok(resp
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|i| TaskList { id: i.id, title: i.title })
            .collect())
    }

    fn tasks(&mut self, list_id: &str) -> Result<Vec<Task>, String> {
        let mut reg = GoogleAuthRegistry::load()?;
        let token = reg.ensure_token(TASKS_SCOPE)?;

        #[derive(Deserialize)]
        struct Response {
            items: Option<Vec<Item>>,
        }

        #[derive(Deserialize)]
        struct Item {
            id: String,
            title: Option<String>,
            status: Option<String>,
        }

        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks?showCompleted=true&maxResults=50",
            list_id
        );

        let resp: Response = api_get(&self.http_client, &url, &token)?;

        Ok(resp
            .items
            .unwrap_or_default()
            .into_iter()
            .filter_map(|i| {
                let title = i.title?;
                if title.is_empty() {
                    return None;
                }
                Some(Task {
                    id: i.id,
                    title,
                    done: i.status.as_deref() == Some("completed"),
                    provider: "google".to_string(),
                })
            })
            .collect())
    }

    fn add_task(&mut self, list_id: &str, title: &str) -> Result<Task, String> {
        let mut reg = GoogleAuthRegistry::load()?;
        let token = reg.ensure_token(TASKS_SCOPE)?;

        #[derive(Serialize)]
        struct NewTask {
            title: String,
        }

        #[derive(Deserialize)]
        struct CreatedTask {
            id: String,
            title: Option<String>,
        }

        let url = format!("https://tasks.googleapis.com/tasks/v1/lists/{}/tasks", list_id);

        let created: CreatedTask = api_post(&self.http_client, &url, &token, &NewTask { title: title.to_string() })?;

        info!("[tasks] Added: {}", title);

        Ok(Task {
            id: created.id,
            title: created.title.unwrap_or_else(|| title.to_string()),
            done: false,
            provider: "google".to_string(),
        })
    }

    fn toggle_task(&mut self, list_id: &str, task_id: &str, done: bool) -> Result<(), String> {
        let mut reg = GoogleAuthRegistry::load()?;
        let token = reg.ensure_token(TASKS_SCOPE)?;

        #[derive(Serialize)]
        struct PatchBody {
            status: String,
        }

        let url = format!("https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}", list_id, task_id);

        let status = if done { "completed" } else { "needsAction" };

        let _: serde_json::Value = api_patch(&self.http_client, &url, &token, &PatchBody { status: status.to_string() })?;

        info!("[tasks] Toggled {} -> {}", task_id, done);
        Ok(())
    }

    fn delete_task(&mut self, list_id: &str, task_id: &str) -> Result<(), String> {
        let mut reg = GoogleAuthRegistry::load()?;
        let token = reg.ensure_token(TASKS_SCOPE)?;

        let url = format!("https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}", list_id, task_id);

        crate::services::tasks::utils::api_delete(&self.http_client, &url, &token)?;

        info!("[tasks] Deleted: {}", task_id);
        Ok(())
    }

    // ── Optimistic UI: Google Tasks spawns its own background threads ───

    fn optimistic_add(&mut self, list_id: &str, title: &str) -> Option<super::provider::Task> {
        let placeholder = super::provider::Task {
            id: String::new(),
            title: title.to_string(),
            done: false,
            provider: "remote".to_string(),
        };

        let client = self.http_client.clone();
        let list_id = list_id.to_string();
        let title = title.to_string();
        std::thread::spawn(move || {
            match GoogleAuthRegistry::load().and_then(|mut r| r.ensure_token(TASKS_SCOPE)) {
                Ok(token) => {
                    let url = format!("https://tasks.googleapis.com/tasks/v1/lists/{}/tasks", list_id);
                    let _ = crate::services::tasks::utils::api_post::<serde_json::Value>(
                        &client, &url, &token,
                        &serde_json::json!({ "title": title }),
                    );
                    log::info!("[tasks] Background add completed: {}", title);
                }
                Err(e) => log::warn!("[tasks] Background add failed: {e}"),
            }
        });

        Some(placeholder)
    }

    fn optimistic_toggle(&mut self, list_id: &str, task_id: &str, done: bool) {
        let client = self.http_client.clone();
        let list_id = list_id.to_string();
        let task_id = task_id.to_string();
        std::thread::spawn(move || {
            match GoogleAuthRegistry::load().and_then(|mut r| r.ensure_token(TASKS_SCOPE)) {
                Ok(token) => {
                    let url = format!("https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}", list_id, task_id);
                    let status = if done { "completed" } else { "needsAction" };
                    let _ = crate::services::tasks::utils::api_patch::<serde_json::Value>(
                        &client, &url, &token,
                        &serde_json::json!({ "status": status }),
                    );
                    log::info!("[tasks] Background toggle completed: {}", task_id);
                }
                Err(e) => log::warn!("[tasks] Background toggle failed: {e}"),
            }
        });
    }

    fn optimistic_delete(&mut self, list_id: &str, task_id: &str) {
        let client = self.http_client.clone();
        let list_id = list_id.to_string();
        let task_id = task_id.to_string();
        std::thread::spawn(move || {
            match GoogleAuthRegistry::load().and_then(|mut r| r.ensure_token(TASKS_SCOPE)) {
                Ok(token) => {
                    let url = format!("https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}", list_id, task_id);
                    let _ = crate::services::tasks::utils::api_delete(&client, &url, &token);
                    log::info!("[tasks] Background delete completed: {}", task_id);
                }
                Err(e) => log::warn!("[tasks] Background delete failed: {e}"),
            }
        });
    }
}
