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
//
// Auth methods have default implementations so local providers
// don't need to implement them. Only remote providers override them.

pub trait TaskProvider: Send + Sync {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;

    /// Whether the provider uses async/background operations.
    /// Async providers get optimistic UI + thread-spawn pattern.
    fn is_async(&self) -> bool {
        false
    }

    // ── Auth (defaults: always authenticated) ──────────────────────────

    fn auth_status(&mut self) -> AuthStatus {
        AuthStatus::Authenticated
    }

    fn authenticate(&mut self) -> Result<AuthStatus, String> {
        Ok(AuthStatus::Authenticated)
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    // ── CRUD ───────────────────────────────────────────────────────────

    fn lists(&mut self) -> Result<Vec<TaskList>, String>;
    fn tasks(&mut self, list_id: &str) -> Result<Vec<Task>, String>;
    fn add_task(&mut self, list_id: &str, title: &str) -> Result<Task, String>;
    fn toggle_task(&mut self, list_id: &str, task_id: &str, done: bool) -> Result<(), String>;
    fn delete_task(&mut self, list_id: &str, task_id: &str) -> Result<(), String>;
    }
