use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LayoutError {
    #[error("Provider error: {0}")]
    ProviderError(String),
}

#[async_trait]
pub trait LayoutProvider: Send + Sync {
    async fn set_active_border_color(&self, color_hex: String) -> Result<(), LayoutError>;
}
