use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq)]
pub enum LauncherAction {
    Exec(String),
    OpenUrl(String),
    Internal(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LauncherItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub icon_name: String,
    pub action: LauncherAction,
    pub score: i32,
}

pub trait LauncherProvider: Debug + Send + Sync {
    fn id(&self) -> &str;
    
    // Manuelle Definition des asynchronen Verhaltens für Dynamic Dispatch (dyn)
    fn search<'a>(
        &'a self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<LauncherItem>> + Send + 'a>>;
}
