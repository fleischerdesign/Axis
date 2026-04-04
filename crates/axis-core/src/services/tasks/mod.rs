pub mod google;
pub mod local;
pub mod provider;
pub mod utils;

pub use provider::{AuthStatus, Task, TaskList, TaskProvider};

pub struct TaskRegistry {
    providers: Vec<Box<dyn TaskProvider>>,
    active: usize,
    cached_tasks: Vec<Task>,
    cached_lists: Vec<TaskList>,
    last_list_id: Option<String>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        let mut providers: Vec<Box<dyn TaskProvider>> = Vec::new();

        // Local provider (always available)
        providers.push(Box::new(local::LocalTodoProvider::new()));

        // Google (always available - will check auth at runtime)
        providers.push(Box::new(google::GoogleTasksProvider::new()));
        let active = 1;

        Self {
            providers,
            active,
            cached_tasks: Vec::new(),
            cached_lists: Vec::new(),
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

    pub fn cached_lists(&self) -> &[TaskList] {
        &self.cached_lists
    }

    pub fn update_cached_task(&mut self, task_id: &str, done: bool) {
        if let Some(t) = self.cached_tasks.iter_mut().find(|t| t.id == task_id) {
            t.done = done;
        }
    }

    pub fn remove_cached_task(&mut self, task_id: &str) {
        self.cached_tasks.retain(|t| t.id != task_id);
    }

    pub fn last_list_id(&self) -> Option<&str> {
        self.last_list_id.as_deref()
    }

    // ── Refresh ────────────────────────────────────────────────────────

    pub fn refresh_tasks(&mut self) -> Result<Vec<Task>, String> {
        self.cached_lists = self.providers[self.active].lists()?;

        let list_id = if let Some(last) = &self.last_list_id {
            if self.cached_lists.iter().any(|l| &l.id == last) {
                last.clone()
            } else {
                self.cached_lists.first().map(|l| l.id.clone()).unwrap_or_else(|| "default".to_string())
            }
        } else {
            self.cached_lists.first().map(|l| l.id.clone()).unwrap_or_else(|| "default".to_string())
        };

        let tasks = self.providers[self.active].tasks(&list_id)?;
        self.cached_tasks = tasks.clone();
        self.last_list_id = Some(list_id);
        Ok(tasks)
    }

    pub fn switch_list(&mut self, list_id: &str) -> Result<Vec<Task>, String> {
        let tasks = self.providers[self.active].tasks(list_id)?;
        self.cached_tasks = tasks.clone();
        self.last_list_id = Some(list_id.to_string());
        Ok(tasks)
    }

    // ── Optimistic Add (encapsulates local-vs-remote branching) ────────

    pub fn optimistic_add_task(&mut self, title: &str) -> Option<Task> {
        let list_id = self.last_list_id().unwrap_or("default").to_string();
        let task = self.providers[self.active].optimistic_add(&list_id, title);
        if let Some(ref t) = task {
            self.cached_tasks.push(t.clone());
        }
        task
    }

    pub fn optimistic_toggle_task(&mut self, task_id: &str, done: bool) {
        self.update_cached_task(task_id, done);
        let list_id = self.last_list_id().unwrap_or("default").to_string();
        self.providers[self.active].optimistic_toggle(&list_id, task_id, done);
    }

    pub fn optimistic_delete_task(&mut self, task_id: &str) {
        self.remove_cached_task(task_id);
        let list_id = self.last_list_id().unwrap_or("default").to_string();
        self.providers[self.active].optimistic_delete(&list_id, task_id);
    }
}
