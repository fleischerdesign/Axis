use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider};
use std::sync::Arc;
use log::info;

pub struct SetFontUseCase {
    provider: Arc<dyn AppearanceProvider>,
}

impl SetFontUseCase {
    pub fn new(provider: Arc<dyn AppearanceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, font: String) -> Result<(), AppearanceError> {
        info!("[use-case] Setting system font to: {}", font);
        self.provider.set_font(Some(font)).await
    }
}
