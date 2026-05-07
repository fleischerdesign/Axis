use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IpcCommand {
    #[default]
    Lock,
    ToggleLauncher,
}
