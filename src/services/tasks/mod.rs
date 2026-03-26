pub mod google;
pub mod local;
pub mod provider;

pub use provider::{AuthStatus, Task, TaskList, TaskProvider};

pub struct TaskRegistry {
    providers: Vec<Box<dyn TaskProvider>>,
    active: usize,
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
        }
    }

    pub fn active(&self) -> &dyn TaskProvider {
        self.providers[self.active].as_ref()
    }

    pub fn active_mut(&mut self) -> &mut dyn TaskProvider {
        self.providers[self.active].as_mut()
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
