pub mod server;

use crate::services::ipc::server::{ShellIpcServer, ShellIpcCmd};
use async_channel::{bounded, Receiver};
use zbus::connection::Builder;
use log::{error, info};

pub struct IpcService;

impl IpcService {
    /// Startet den D-Bus IPC Server und gibt den Command-Receiver zurück
    pub fn spawn() -> Receiver<ShellIpcCmd> {
        let (tx, rx) = bounded(32);
        let server = ShellIpcServer::new(tx);

        tokio::spawn(async move {
            let conn_res = async {
                let builder = Builder::session()?;
                let builder = builder.name("org.axis.Shell")?;
                let builder = builder.serve_at("/org/axis/Shell", server)?;
                builder.build().await
            }.await;

            match conn_res {
                Ok(_conn) => {
                    info!("[ipc] D-Bus Interface 'org.axis.Shell' registered and active");
                    std::future::pending::<()>().await;
                }
                Err(e) => error!("[ipc] Failed to register D-Bus interface: {:?}", e),
            }
        });

        rx
    }
}
