pub mod server;

use crate::services::ipc::server::{ShellIpcServer, ShellIpcCmd};
use async_channel::{bounded, Receiver};

pub struct IpcService;

impl IpcService {
    /// Creates the IPC server and the command receiver.
    /// D-Bus registration must be performed externally.
    pub fn create_server() -> (ShellIpcServer, Receiver<ShellIpcCmd>) {
        let (tx, rx) = bounded(32);
        let server = ShellIpcServer::new(tx);
        (server, rx)
    }
}
