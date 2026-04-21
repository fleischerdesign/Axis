use crate::models::popups::{PopupType, PopupStatus};
use async_trait::async_trait;
use thiserror::Error;
use futures_util::Stream;
use std::pin::Pin;

#[derive(Error, Debug)]
pub enum PopupError {
    #[error("Popup provider error: {0}")]
    ProviderError(String),
}

pub type PopupStream = Pin<Box<dyn Stream<Item = PopupStatus> + Send>>;

#[async_trait]
pub trait PopupProvider: Send + Sync {
    async fn get_status(&self) -> Result<PopupStatus, PopupError>;
    async fn subscribe(&self) -> Result<PopupStream, PopupError>;
    async fn open_popup(&self, popup_type: PopupType) -> Result<(), PopupError>;
    async fn close_popup(&self) -> Result<(), PopupError>;
    async fn toggle_popup(&self, popup_type: PopupType) -> Result<(), PopupError>;
}
