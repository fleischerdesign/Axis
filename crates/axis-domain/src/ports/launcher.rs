use crate::models::launcher::LauncherItem;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LauncherError {
    #[error("Launcher search error: {0}")]
    SearchError(String),
}

#[async_trait]
pub trait LauncherSearchProvider: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError>;
}
