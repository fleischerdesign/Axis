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
    NeedsAuth { url: String, code: Option<String> },
    Failed(String),
}

// ── TaskProvider Trait ────────────────────────────────────────────────
//
// Auth methods have default implementations so local providers
// don't need to implement them. Only remote providers override them.

pub trait TaskProvider: Send + Sync {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;

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

    // ── Optimistic UI (defaults: sync path, override for async) ────────

    fn optimistic_add(&mut self, list_id: &str, title: &str) -> Option<Task> {
        match self.add_task(list_id, title) {
            Ok(task) => Some(task),
            Err(e) => {
                log::warn!("[tasks] add_task failed: {e}");
                None
            }
        }
    }

    fn optimistic_toggle(&mut self, list_id: &str, task_id: &str, done: bool) {
        let _ = self.toggle_task(list_id, task_id, done);
    }

    fn optimistic_delete(&mut self, list_id: &str, task_id: &str) {
        let _ = self.delete_task(list_id, task_id);
    }
}
