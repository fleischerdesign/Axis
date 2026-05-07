use axis_domain::models::launcher::LauncherItem;
use axis_domain::ports::launcher::{LauncherError, LauncherSearchProvider};
use log::debug;
use std::sync::Arc;

pub struct SearchLauncherUseCase {
    provider: Arc<dyn LauncherSearchProvider>,
}

impl SearchLauncherUseCase {
    pub fn new(provider: Arc<dyn LauncherSearchProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError> {
        debug!("[use-case] Searching launcher for: '{}'", query);
        let results = self.provider.search(query).await?;
        debug!(
            "[use-case] Launcher search returned {} results",
            results.len()
        );
        Ok(results)
    }
}
