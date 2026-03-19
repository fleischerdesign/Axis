pub mod server;

use crate::services::ipc::server::{ShellIpcServer, ShellIpcCmd};
use async_channel::{bounded, Receiver};
use zbus::connection::Builder;

pub struct IpcService;

impl IpcService {
    /// Startet den D-Bus IPC Server und gibt den Command-Receiver zurück
    pub fn spawn() -> Receiver<ShellIpcCmd> {
        let (tx, rx) = bounded(32);
        let server = ShellIpcServer::new(tx);

        tokio::spawn(async move {
            let conn_res = Builder::session()
                .unwrap()
                .name("org.axis.Shell")
                .unwrap()
                .serve_at("/org/axis/Shell", server)
                .unwrap()
                .build()
                .await;

            match conn_res {
                Ok(_conn) => {
                    println!("IPC: D-Bus Interface 'org.axis.Shell' registered and active");
                    // WICHTIG: Wir müssen die Verbindung halten!
                    // Solange dieser Future läuft, bleibt die Verbindung offen.
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                    }
                }
                Err(e) => eprintln!("IPC: Failed to register D-Bus interface: {:?}", e),
            }
        });

        rx
    }
}
