use crate::models::notifications::NotificationStatus;
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Notification service error: {0}")]
    ServiceError(String),
}

pub type NotificationStream = Pin<Box<dyn Stream<Item = NotificationStatus> + Send>>;

#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn get_status(&self) -> Result<NotificationStatus, NotificationError>;
    async fn subscribe(&self) -> Result<NotificationStream, NotificationError>;
    async fn close_notification(&self, id: u32) -> Result<(), NotificationError>;
    async fn invoke_action(&self, id: u32, action_key: &str) -> Result<(), NotificationError>;
}
