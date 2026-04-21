use axis_domain::ports::tray::{TrayProvider, TrayError};
use std::sync::Arc;

pub struct ActivateTrayItemUseCase {
    provider: Arc<dyn TrayProvider>,
}

impl ActivateTrayItemUseCase {
    pub fn new(provider: Arc<dyn TrayProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, bus_name: &str, x: i32, y: i32) -> Result<(), TrayError> {
        self.provider.activate(bus_name, x, y).await
    }
}
