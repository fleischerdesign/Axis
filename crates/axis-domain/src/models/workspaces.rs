use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub id: u32,
    pub name: String,
    pub is_active: bool,
    pub is_empty: bool,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceStatus {
    pub workspaces: Vec<Workspace>,
    pub overview_open: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_status_serde_roundtrip() {
        let status = WorkspaceStatus {
            workspaces: vec![Workspace {
                id: 1,
                name: "1".into(),
                is_active: true,
                is_empty: false,
                index: 0,
            }],
            overview_open: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: WorkspaceStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }

    #[test]
    fn workspace_status_default_overview_closed() {
        let status = WorkspaceStatus {
            workspaces: vec![],
            overview_open: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: WorkspaceStatus = serde_json::from_str(&json).unwrap();
        assert!(!back.overview_open);
    }
}
