use crate::models::config::AxisConfig;
use super::StatusStream;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ConfigError {
    #[error("Config provider error: {0}")]
    ProviderError(String),
}

pub type ConfigStream = StatusStream<AxisConfig>;

pub trait ConfigProvider: Send + Sync {
    fn get(&self) -> Result<AxisConfig, ConfigError>;
    fn subscribe(&self) -> Result<ConfigStream, ConfigError>;
    fn update(&self, apply: Box<dyn FnOnce(&mut AxisConfig) + Send + 'static>) -> Result<(), ConfigError>;
}
