use crate::models::appearance::{AccentColor, AppearanceStatus, ColorScheme};
use async_trait::async_trait;
use futures_util::Stream;
use std::pin::Pin;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppearanceError {
    #[error("Appearance provider error: {0}")]
    ProviderError(String),
}

pub type AppearanceStream = Pin<Box<dyn Stream<Item = AppearanceStatus> + Send>>;

#[async_trait]
pub trait AppearanceProvider: Send + Sync {
    async fn get_status(&self) -> Result<AppearanceStatus, AppearanceError>;
    async fn subscribe(&self) -> Result<AppearanceStream, AppearanceError>;
    async fn set_wallpaper(&self, path: Option<String>) -> Result<(), AppearanceError>;
    async fn set_accent_color(&self, color: AccentColor) -> Result<(), AppearanceError>;
    async fn set_color_scheme(&self, scheme: ColorScheme) -> Result<(), AppearanceError>;
    async fn set_font(&self, font: Option<String>) -> Result<(), AppearanceError>;
}
