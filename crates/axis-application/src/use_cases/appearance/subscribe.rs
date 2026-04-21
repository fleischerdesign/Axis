use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider, AppearanceStream};
use std::sync::Arc;

pub struct SubscribeToAppearanceUseCase {
    provider: Arc<dyn AppearanceProvider>,
}

impl SubscribeToAppearanceUseCase {
    pub fn new(provider: Arc<dyn AppearanceProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self) -> Result<AppearanceStream, AppearanceError> {
        self.provider.subscribe().await
    }
}
