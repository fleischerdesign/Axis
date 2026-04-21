use crate::models::ipc::IpcCommand;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("IPC error: {0}")]
    ProviderError(String),
}

#[async_trait]
pub trait IpcProvider: Send + Sync {
    async fn run(&self, on_command: Box<dyn Fn(IpcCommand) + Send + Sync>) -> Result<(), IpcError>;
}
