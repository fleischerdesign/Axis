use axis_domain::models::ipc::IpcCommand;
use axis_domain::ports::ipc::{IpcError, IpcProvider};
use async_trait::async_trait;
use std::sync::Arc;
use zbus::connection;

struct ShellIface {
    on_command: Arc<dyn Fn(IpcCommand) + Send + Sync>,
}

#[zbus::interface(name = "org.axis.Shell")]
impl ShellIface {
    async fn toggle_launcher(&self) -> zbus::fdo::Result<()> {
        (self.on_command)(IpcCommand::ToggleLauncher);
        Ok(())
    }

    async fn lock(&self) -> zbus::fdo::Result<()> {
        (self.on_command)(IpcCommand::Lock);
        Ok(())
    }
}

pub struct ZbusIpcProvider;

impl ZbusIpcProvider {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl IpcProvider for ZbusIpcProvider {
    async fn run(&self, on_command: Box<dyn Fn(IpcCommand) + Send + Sync>) -> Result<(), IpcError> {
        let on_command = Arc::from(on_command);
        let iface = ShellIface { on_command };

        let _conn = connection::Builder::session()
            .map_err(|e| IpcError::ProviderError(e.to_string()))?
            .name("org.axis.Shell")
            .map_err(|e| IpcError::ProviderError(e.to_string()))?
            .serve_at("/org/axis/Shell", iface)
            .map_err(|e| IpcError::ProviderError(e.to_string()))?
            .build()
            .await
            .map_err(|e| IpcError::ProviderError(e.to_string()))?;

        log::info!("[ipc] D-Bus server started on org.axis.Shell");

        std::future::pending::<()>().await;
        Ok(())
    }
}
