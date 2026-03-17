use crate::services::launcher::provider::{LauncherItem, LauncherProvider};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Default)]
pub struct AppProvider;

impl LauncherProvider for AppProvider {
    fn id(&self) -> &str {
        "apps"
    }

    fn search<'a>(
        &'a self,
        _query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<LauncherItem>> + Send + 'a>> {
        Box::pin(async move {
            // Später: Desktop-Dateien parsen
            Vec::new()
        })
    }
}
