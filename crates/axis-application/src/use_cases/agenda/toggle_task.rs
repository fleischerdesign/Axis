use axis_domain::ports::agenda::{AgendaProvider, AgendaError};
use std::sync::Arc;
use log::info;

pub struct ToggleTaskUseCase {
    provider: Arc<dyn AgendaProvider>,
}

impl ToggleTaskUseCase {
    pub fn new(provider: Arc<dyn AgendaProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(
        &self,
        list_id: &str,
        task_id: &str,
        done: bool,
    ) -> Result<(), AgendaError> {
        info!("[use-case] Toggling task {} to done={}", task_id, done);
        self.provider.toggle_task(list_id, task_id, done).await
    }
}
