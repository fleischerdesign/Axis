use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CloudAccount {
    pub id: String,
    pub provider_name: String,
    pub display_name: String,
    pub status: AccountStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AccountStatus {
    Online,
    Offline,
    NeedsAuthentication(String),
    Error(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CloudStatus {
    pub accounts: Vec<CloudAccount>,
}
