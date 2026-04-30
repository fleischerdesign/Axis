use crate::models::popups::{PopupType, PopupStatus};
use async_trait::async_trait;
use thiserror::Error;
use super::StatusStream;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum PopupError {
    #[error("Popup provider error: {0}")]
    ProviderError(String),
}

pub type PopupStream = StatusStream<PopupStatus>;

#[async_trait]
pub trait PopupProvider: Send + Sync {
    async fn get_status(&self) -> Result<PopupStatus, PopupError>;
    async fn subscribe(&self) -> Result<PopupStream, PopupError>;
    async fn open_popup(&self, popup_type: PopupType) -> Result<(), PopupError>;
    async fn close_popup(&self) -> Result<(), PopupError>;
    async fn toggle_popup(&self, popup_type: PopupType) -> Result<(), PopupError>;
}

crate::status_provider!(PopupProvider, PopupStatus, PopupError);
