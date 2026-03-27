pub mod google;
pub mod local;
pub mod provider;
pub mod utils;

pub use provider::{AuthStatus, Task, TaskProvider};

pub struct TaskRegistry {
    providers: Vec<Box<dyn TaskProvider>>,
    active: usize,
    cached_tasks: Vec<Task>,
    last_list_id: Option<String>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        let mut providers: Vec<Box<dyn TaskProvider>> = Vec::new();

        // Local provider (always available)
        providers.push(Box::new(local::LocalTodoProvider::new()));

        // Google (only if credentials exist)
        let mut active = 0;
        match google::GoogleTasksProvider::load() {
            Ok(google) => {
                providers.push(Box::new(google));
                active = 1; // Default to Google Tasks
            }
            Err(e) => log::info!("[tasks] Google provider unavailable: {e}"),
        }

        Self {
            providers,
            active,
            cached_tasks: Vec::new(),
            last_list_id: None,
        }
    }

    // ── Provider Access (bounds-safe) ──────────────────────────────────

    pub fn active(&self) -> &dyn TaskProvider {
        &*self.providers[self.active]
    }

    pub fn active_mut(&mut self) -> &mut dyn TaskProvider {
        &mut *self.providers[self.active]
    }

    // ── Cache ──────────────────────────────────────────────────────────

    pub fn cached_tasks(&self) -> &[Task] {
        &self.cached_tasks
    }

    pub fn update_cached_task(&mut self, task_id: &str, done: bool) {
        if let Some(t) = self.cached_tasks.iter_mut().find(|t| t.id == task_id) {
            t.done = done;
        }
    }

    pub fn last_list_id(&self) -> Option<&str> {
        self.last_list_id.as_deref()
    }

    // ── Refresh ────────────────────────────────────────────────────────

    pub fn refresh_tasks(&mut self) -> Result<Vec<Task>, String> {
        let lists = self.providers[self.active].lists()?;
        let list_id = lists.first().map(|l| l.id.as_str()).unwrap_or("default");
        let tasks = self.providers[self.active].tasks(list_id)?;
        self.cached_tasks = tasks.clone();
        self.last_list_id = Some(list_id.to_string());
        Ok(tasks)
    }

    // ── Optimistic Add (encapsulates local-vs-remote branching) ────────

    pub fn optimistic_add_task(&mut self, title: &str) -> Option<Task> {
        if self.providers[self.active].is_async() {
            // Async provider: add placeholder to cache, caller handles API
            let placeholder = Task {
                id: String::new(),
                title: title.to_string(),
                done: false,
                provider: "remote".to_string(),
            };
            self.cached_tasks.push(placeholder.clone());
            Some(placeholder)
        } else {
            // Sync provider: do it now
            let list_id = self.last_list_id().unwrap_or("default").to_string();
            match self.providers[self.active].add_task(&list_id, title) {
                Ok(task) => {
                    self.cached_tasks.push(task.clone());
                    Some(task)
                }
                Err(e) => {
                    log::warn!("[tasks] add_task failed: {e}");
                    None
                }
            }
        }
    }

    pub fn optimistic_toggle_task(&mut self, task_id: &str, done: bool) {
        self.update_cached_task(task_id, done);

        if self.providers[self.active].is_async() {
            // Async: caller must spawn API call separately
        } else {
            let list_id = self.last_list_id().unwrap_or("default").to_string();
            let _ = self.providers[self.active].toggle_task(&list_id, task_id, done);
        }
    }
}
