#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchPriority {
    Fallback = 0,
    Primary = 1,
}

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
    pub priority: SearchPriority,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LauncherStatus {
    pub query: String,
    pub results: Vec<LauncherItem>,
    pub selected_index: Option<usize>,
    pub is_searching: bool,
}

impl Default for LauncherStatus {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected_index: None,
            is_searching: false,
        }
    }
}
