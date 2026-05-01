pub mod apps;
pub mod files;
pub mod web;
pub mod util;

use axis_domain::models::launcher::LauncherItem;
use axis_domain::ports::launcher::{LauncherError, LauncherSearchProvider};
use async_trait::async_trait;
use log::warn;
use std::sync::Arc;

use self::apps::AppSearchProvider;
use self::files::FileSearchProvider;
use self::web::WebSearchProvider;

pub struct CompositeLauncherProvider {
    providers: Vec<Arc<dyn LauncherSearchProvider>>,
}

impl CompositeLauncherProvider {
    pub fn new() -> Arc<Self> {
        let providers: Vec<Arc<dyn LauncherSearchProvider>> = vec![
            AppSearchProvider::new(),
            Arc::new(FileSearchProvider),
            WebSearchProvider::new(),
        ];
        Arc::new(Self { providers })
    }
}

#[async_trait]
impl LauncherSearchProvider for CompositeLauncherProvider {
    async fn search(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError> {
        let query = query.trim().to_string();
        if query.is_empty() {
            let mut all = Vec::new();
            for p in &self.providers {
                match p.search("").await {
                    Ok(results) => all.extend(results),
                    Err(e) => warn!("[launcher] Provider error: {e}"),
                }
            }
            all.sort_by(|a, b| {
                b.priority.cmp(&a.priority).then_with(|| b.score.cmp(&a.score))
            });
            return Ok(all);
        }

        let mut handles = Vec::new();
        for p in &self.providers {
            let p = p.clone();
            let q = query.clone();
            handles.push(tokio::spawn(async move { p.search(&q).await }));
        }

        let mut all_results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(results)) => all_results.extend(results),
                Ok(Err(e)) => warn!("[launcher] Provider error: {e}"),
                Err(e) => warn!("[launcher] Provider task panicked: {e}"),
            }
        }

        all_results.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then_with(|| b.score.cmp(&a.score))
        });

        Ok(all_results)
    }
}
