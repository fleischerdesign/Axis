use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider};
use std::sync::Arc;

pub struct SetWallpaperUseCase {
    provider: Arc<dyn AppearanceProvider>,
}

impl SetWallpaperUseCase {
    pub fn new(provider: Arc<dyn AppearanceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, path: Option<String>) -> Result<(), AppearanceError> {
        self.provider.set_wallpaper(path).await
    }
}
