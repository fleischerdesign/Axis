use axis_domain::models::launcher::LauncherItem;
use axis_domain::ports::launcher::{LauncherError, LauncherSearchProvider};
use std::sync::Arc;

pub struct SearchLauncherUseCase {
    provider: Arc<dyn LauncherSearchProvider>,
}

impl SearchLauncherUseCase {
    pub fn new(provider: Arc<dyn LauncherSearchProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError> {
        self.provider.search(query).await
    }
}
