use axis_domain::models::launcher::LauncherItem;
use axis_domain::ports::launcher::{LauncherError, LauncherSearchProvider};
use std::sync::Arc;
use std::time::Instant;
use log::{debug, info};

pub struct SearchLauncherUseCase {
    provider: Arc<dyn LauncherSearchProvider>,
}

impl SearchLauncherUseCase {
    pub fn new(provider: Arc<dyn LauncherSearchProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError> {
        let start = Instant::now();
        
        debug!("[use-case] Searching launcher for: '{}'", query);
        
        let results = self.provider.search(query).await?;
        
        let duration = start.elapsed();
        debug!("[use-case] Launcher search returned {} results in {:?}", results.len(), duration);

        if duration.as_millis() > 100 {
            info!("[use-case] Slow launcher search detected: {:?} for query '{}'", duration, query);
        }

        Ok(results)
    }
}
