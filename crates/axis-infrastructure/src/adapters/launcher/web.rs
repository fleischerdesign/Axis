use axis_domain::models::launcher::{LauncherAction, LauncherItem, SearchPriority};
use axis_domain::ports::launcher::{LauncherError, LauncherSearchProvider};
use async_trait::async_trait;
use std::sync::Arc;

fn url_encode(query: &str) -> String {
    query
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
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

pub struct WebSearchProvider;

impl WebSearchProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    fn build_result(&self, query: &str) -> Option<LauncherItem> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return None;
        }

        let url = format!(
            "https://www.google.com/search?q={}",
            url_encode(trimmed)
        );

        Some(LauncherItem {
            id: "web-google".to_string(),
            title: format!("Google: {trimmed}"),
            description: Some("Search the web".into()),
            icon_name: "web-browser-symbolic".into(),
            action: LauncherAction::OpenUrl(url),
            score: 80,
            priority: SearchPriority::Fallback,
        })
    }
}

#[async_trait]
impl LauncherSearchProvider for WebSearchProvider {
    async fn search(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError> {
        Ok(self.build_result(query).into_iter().collect())
    }
}
