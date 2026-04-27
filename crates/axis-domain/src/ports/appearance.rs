use crate::models::appearance::{AccentColor, ColorScheme};
use crate::models::config::AppearanceConfig;
use async_trait::async_trait;
use super::StatusStream;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppearanceError {
    #[error("Appearance provider error: {0}")]
    ProviderError(String),
}

pub type AppearanceStream = StatusStream<AppearanceConfig>;

#[async_trait]
pub trait AppearanceProvider: Send + Sync {
    async fn get_status(&self) -> Result<AppearanceConfig, AppearanceError>;
    async fn subscribe(&self) -> Result<AppearanceStream, AppearanceError>;
    async fn set_wallpaper(&self, path: Option<String>) -> Result<(), AppearanceError>;
    async fn set_accent_color(&self, color: AccentColor) -> Result<(), AppearanceError>;
    async fn set_color_scheme(&self, scheme: ColorScheme) -> Result<(), AppearanceError>;
    async fn set_font(&self, font: Option<String>) -> Result<(), AppearanceError>;
}
