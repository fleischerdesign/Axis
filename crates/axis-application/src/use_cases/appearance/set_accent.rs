use axis_domain::models::appearance::AccentColor;
use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider};
use axis_domain::ports::layout::LayoutProvider;
use std::sync::Arc;
use log::{info, warn};

pub struct SetAccentColorUseCase {
    appearance_port: Arc<dyn AppearanceProvider>,
    layout_port: Arc<dyn LayoutProvider>,
}

impl SetAccentColorUseCase {
    pub fn new(
        appearance_port: Arc<dyn AppearanceProvider>,
        layout_port: Arc<dyn LayoutProvider>
    ) -> Self {
        Self { appearance_port, layout_port }
    }

    pub async fn execute(&self, color: AccentColor) -> Result<(), AppearanceError> {
        info!("[use-case] Setting accent color to: {:?}", color);

        // 1. Domain Validation
        let hex = color.hex_value().to_string();
        if !AccentColor::is_valid_hex(&hex) {
            return Err(AppearanceError::ProviderError(format!("Invalid hex color: {}", hex)));
        }

        // 2. Persist in Config
        self.appearance_port.set_accent_color(color).await?;

        // 3. Orchestrate Compositor Sync
        if let Err(e) = self.layout_port.set_active_border_color(hex).await {
            warn!("[use-case] Failed to sync accent color with compositor: {}", e);
        }

        Ok(())
    }
}
