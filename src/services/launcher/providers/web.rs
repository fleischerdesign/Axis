use crate::services::launcher::provider::{LauncherAction, LauncherItem, LauncherProvider, SearchPriority};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone)]
struct SearchEngine {
    name: &'static str,
    url_template: &'static str,
}

const DEFAULT_ENGINE: SearchEngine = SearchEngine {
    name: "Google",
    url_template: "https://www.google.com/search?q={query}",
};

fn url_encode(query: &str) -> String {
    query
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                c.to_string()
            }
            ' ' => "+".to_string(),
            _ => {
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                encoded
                    .bytes()
                    .map(|b| format!("%{b:02X}"))
                    .collect::<String>()
            }
        })
        .collect()
}

#[derive(Debug)]
pub struct WebSearchProvider {
    engine: SearchEngine,
}

impl Default for WebSearchProvider {
    fn default() -> Self {
        Self {
            engine: DEFAULT_ENGINE,
        }
    }
}

impl WebSearchProvider {
    fn build_result(&self, query: &str) -> Option<LauncherItem> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return None;
        }

        let url = self
            .engine
            .url_template
            .replace("{query}", &url_encode(trimmed));

        Some(LauncherItem {
            id: format!("web-{}", self.engine.name.to_lowercase()),
            title: format!("{}: {}", self.engine.name, trimmed),
            description: Some("Im Web suchen".into()),
            icon_name: "web-browser-symbolic".into(),
            action: LauncherAction::OpenUrl(url),
            score: 80,
            priority: SearchPriority::Fallback,
        })
    }
}

impl LauncherProvider for WebSearchProvider {
    fn id(&self) -> &str {
        "web"
    }

    fn priority(&self) -> SearchPriority {
        SearchPriority::Fallback
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<LauncherItem>> + Send + 'a>> {
        Box::pin(async move { self.build_result(query).into_iter().collect() })
    }
}
