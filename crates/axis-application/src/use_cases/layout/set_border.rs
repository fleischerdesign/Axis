use axis_domain::ports::layout::{LayoutError, LayoutProvider};
use log::debug;
use std::sync::Arc;

pub struct SetBorderColorUseCase {
    provider: Arc<dyn LayoutProvider>,
}

impl SetBorderColorUseCase {
    pub fn new(provider: Arc<dyn LayoutProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, color_hex: String) -> Result<(), LayoutError> {
        debug!("[use-case] Setting active border color to: {}", color_hex);
        self.provider.set_active_border_color(color_hex).await
    }
}
