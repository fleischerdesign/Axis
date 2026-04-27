use crate::models::notifications::NotificationStatus;
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug)]
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
    async fn invoke_action(&self, id: u32, action_key: &str) -> Result<(), NotificationError>;
}
