use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum SearchPriority {
    #[default]
    Fallback = 0,
    Primary = 1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum LauncherAction {
    #[default]
    Noop,
    Exec(Vec<String>),
    OpenUrl(String),
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LauncherItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub icon_name: String,
    pub action: LauncherAction,
    pub score: i32,
    pub priority: SearchPriority,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LauncherStatus {
    pub query: String,
    pub results: Vec<LauncherItem>,
    pub selected_index: Option<usize>,
    pub is_searching: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_action_exec() {
        let a = LauncherAction::Exec(vec!["firefox".into()]);
        match a {
            LauncherAction::Exec(args) => assert_eq!(args, vec!["firefox"]),
            _ => panic!("expected Exec"),
        }
    }

    #[test]
    fn launcher_action_open_url() {
        let a = LauncherAction::OpenUrl("https://example.com".into());
        assert_eq!(format!("{a:?}"), "OpenUrl(\"https://example.com\")");
    }

    #[test]
    fn search_priority_ordering() {
        assert!(SearchPriority::Primary > SearchPriority::Fallback);
    }
}
