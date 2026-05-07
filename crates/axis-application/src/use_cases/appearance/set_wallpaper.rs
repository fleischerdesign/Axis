use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider};
use log::{info, warn};
use std::path::Path;
use std::sync::Arc;

pub struct SetWallpaperUseCase {
    provider: Arc<dyn AppearanceProvider>,
}

impl SetWallpaperUseCase {
    pub fn new(provider: Arc<dyn AppearanceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, path: String) -> Result<(), AppearanceError> {
        info!("[use-case] Setting wallpaper to: {}", path);

        // 1. Domain Validation
        let p = Path::new(&path);
        if !p.exists() {
            warn!("[use-case] Wallpaper path does not exist: {}", path);
            return Err(AppearanceError::ValidationError(format!(
                "File not found: {}",
                path
            )));
        }

        // 2. Persist
        self.provider.set_wallpaper(Some(path)).await
    }
}
