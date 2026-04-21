use axis_domain::models::appearance::AccentColor;
use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider};
use std::sync::Arc;

pub struct SetAccentColorUseCase {
    provider: Arc<dyn AppearanceProvider>,
}

impl SetAccentColorUseCase {
    pub fn new(provider: Arc<dyn AppearanceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, color: AccentColor) -> Result<(), AppearanceError> {
        self.provider.set_accent_color(color).await
    }
}
