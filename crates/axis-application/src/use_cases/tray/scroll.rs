use axis_domain::ports::tray::{TrayProvider, TrayError};
use std::sync::Arc;

pub struct ScrollTrayItemUseCase {
    provider: Arc<dyn TrayProvider>,
}

impl ScrollTrayItemUseCase {
    pub fn new(provider: Arc<dyn TrayProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, bus_name: &str, delta: i32, orientation: &str) -> Result<(), TrayError> {
        self.provider.scroll(bus_name, delta, orientation).await
    }
}
