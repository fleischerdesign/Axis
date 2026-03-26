pub mod google;
pub mod local;
pub mod provider;

pub use provider::{AuthStatus, Task, TaskList, TaskProvider};

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
        match google::GoogleTasksProvider::load() {
            Ok(google) => providers.push(Box::new(google)),
            Err(e) => log::info!("[tasks] Google provider unavailable: {e}"),
        }

        Self {
            providers,
            active: 1,
            cached_tasks: Vec::new(),
            last_list_id: None,
        }
    }

    pub fn active(&self) -> &dyn TaskProvider {
        self.providers[self.active].as_ref()
    }

    pub fn active_mut(&mut self) -> &mut dyn TaskProvider {
        self.providers[self.active].as_mut()
    }

    pub fn cached_tasks(&self) -> &[Task] {
        &self.cached_tasks
    }

    pub fn cached_tasks_mut(&mut self) -> &mut Vec<Task> {
        &mut self.cached_tasks
    }

    pub fn set_cached_tasks(&mut self, tasks: Vec<Task>, list_id: String) {
        self.cached_tasks = tasks;
        self.last_list_id = Some(list_id);
    }

    pub fn update_cached_task(&mut self, task_id: &str, done: bool) {
        if let Some(t) = self.cached_tasks.iter_mut().find(|t| t.id == task_id) {
            t.done = done;
        }
    }

    pub fn last_list_id(&self) -> Option<&str> {
        self.last_list_id.as_deref()
    }

    pub fn refresh_tasks(&mut self) -> Result<Vec<Task>, String> {
        let lists = self.providers[self.active].lists()?;
        let list_id = lists.first().map(|l| l.id.as_str()).unwrap_or("default");
        let tasks = self.providers[self.active].tasks(list_id)?;
        self.cached_tasks = tasks.clone();
        self.last_list_id = Some(list_id.to_string());
        Ok(tasks)
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.providers.len() {
            self.active = index;
        }
    }

    pub fn count(&self) -> usize {
        self.providers.len()
    }

    pub fn provider_name(&self, index: usize) -> Option<&str> {
        self.providers.get(index).map(|p| p.name())
    }
}
