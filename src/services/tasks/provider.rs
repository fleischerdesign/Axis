use serde::{Deserialize, Serialize};

// ── Shared Types ──────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub provider: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskList {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug)]
pub enum AuthStatus {
    Authenticated,
    NeedsAuth { url: String, code: String },
    Failed(String),
}

// ── TaskProvider Trait ────────────────────────────────────────────────

pub trait TaskProvider: Send + Sync {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn is_local(&self) -> bool;
    fn auth_status(&mut self) -> AuthStatus;
    fn authenticate(&mut self) -> Result<AuthStatus, String>;
    fn is_authenticated(&self) -> bool;
    fn lists(&mut self) -> Result<Vec<TaskList>, String>;
    fn tasks(&mut self, list_id: &str) -> Result<Vec<Task>, String>;
    fn add_task(&mut self, list_id: &str, title: &str) -> Result<Task, String>;
    fn toggle_task(&mut self, list_id: &str, task_id: &str, done: bool) -> Result<(), String>;
}
