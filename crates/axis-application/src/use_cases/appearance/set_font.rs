use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider};
use std::sync::Arc;

pub struct SetFontUseCase {
    provider: Arc<dyn AppearanceProvider>,
}

impl SetFontUseCase {
    pub fn new(provider: Arc<dyn AppearanceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, font: Option<String>) -> Result<(), AppearanceError> {
        self.provider.set_font(font).await
    }
}
