use axis_domain::models::appearance::ColorScheme;
use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider};
use std::sync::Arc;
use log::info;

pub struct SetColorSchemeUseCase {
    provider: Arc<dyn AppearanceProvider>,
}

impl SetColorSchemeUseCase {
    pub fn new(provider: Arc<dyn AppearanceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, scheme: ColorScheme) -> Result<(), AppearanceError> {
        info!("[use-case] Setting color scheme to: {:?}", scheme);
        self.provider.set_color_scheme(scheme).await
    }
}
