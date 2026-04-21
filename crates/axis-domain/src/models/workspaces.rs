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
}
