use axis_domain::models::appearance::AppearanceStatus;
use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider};
use std::sync::Arc;

pub struct GetAppearanceStatusUseCase {
    provider: Arc<dyn AppearanceProvider>,
}

impl GetAppearanceStatusUseCase {
    pub fn new(provider: Arc<dyn AppearanceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<AppearanceStatus, AppearanceError> {
        self.provider.get_status().await
    }
}
