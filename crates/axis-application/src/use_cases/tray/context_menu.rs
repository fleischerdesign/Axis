use axis_domain::ports::tray::{TrayProvider, TrayError};
use std::sync::Arc;

pub struct ContextMenuTrayItemUseCase {
    provider: Arc<dyn TrayProvider>,
}

impl ContextMenuTrayItemUseCase {
    pub fn new(provider: Arc<dyn TrayProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, bus_name: &str, x: i32, y: i32) -> Result<(), TrayError> {
        self.provider.context_menu(bus_name, x, y).await
    }
}
