use axis_domain::ports::tray::{TrayProvider, TrayError};
use std::sync::Arc;
use log::debug;

pub struct ScrollTrayItemUseCase {
    provider: Arc<dyn TrayProvider>,
}

impl ScrollTrayItemUseCase {
    pub fn new(provider: Arc<dyn TrayProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, bus_name: &str, delta: i32, orientation: &str) -> Result<(), TrayError> {
        debug!("[use-case] Scrolling tray item: {} (delta={}, orientation={})", bus_name, delta, orientation);
        self.provider.scroll(bus_name, delta, orientation).await
    }
}
