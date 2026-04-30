use crate::models::launcher::LauncherItem;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LauncherError {
    #[error("Launcher provider error: {0}")]
    ProviderError(String),
}

#[async_trait]
pub trait LauncherSearchProvider: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError>;
}
