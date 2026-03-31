use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq)]
pub enum LauncherAction {
    Exec(String),
    OpenUrl(String),
    Internal(String),
}

/// Ergebnis-Priorität. Bestimmt die Reihenfolge in der Ergebnisliste.
/// Primary-Ergebnisse erscheinen immer über Fallback-Ergebnissen,
/// innerhalb der Ebene wird nach Score sortiert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchPriority {
    Fallback = 0,
    Primary = 1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LauncherItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub icon_name: String,
    pub action: LauncherAction,
    pub score: i32,
    pub priority: SearchPriority,
}

pub trait LauncherProvider: Debug + Send + Sync {
    fn id(&self) -> &str;

    fn priority(&self) -> SearchPriority {
        SearchPriority::Primary
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<LauncherItem>> + Send + 'a>>;
}
