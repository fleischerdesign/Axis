use crate::models::notifications::{Notification, NotificationStatus};
use async_trait::async_trait;
use thiserror::Error;
use std::collections::HashMap;
use std::sync::Arc;
use super::StatusStream;

pub type ActionHandler = Arc<dyn Fn(Option<String>) + Send + Sync>;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum NotificationError {
    #[error("Notification provider error: {0}")]
    ProviderError(String),
}

pub type NotificationStream = StatusStream<NotificationStatus>;

#[async_trait]
pub trait NotificationProvider: Send + Sync {
    async fn get_status(&self) -> Result<NotificationStatus, NotificationError>;
    async fn subscribe(&self) -> Result<NotificationStream, NotificationError>;
    async fn close_notification(&self, id: u32) -> Result<(), NotificationError>;
    async fn invoke_action(&self, id: u32, action_key: &str, user_input: Option<String>) -> Result<(), NotificationError>;
    async fn show(
        &self,
        notification: Notification,
        action_handlers: HashMap<String, ActionHandler>,
    ) -> Result<u32, NotificationError>;
}

crate::status_provider!(NotificationProvider, NotificationStatus, NotificationError);
