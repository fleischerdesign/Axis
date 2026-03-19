use zbus::interface;
use async_channel::Sender;

pub enum ShellIpcCmd {
    ToggleLauncher,
    ToggleQuickSettings,
    ToggleWorkspaces,
    CloseAll,
}

pub struct ShellIpcServer {
    cmd_tx: Sender<ShellIpcCmd>,
}

impl ShellIpcServer {
    pub fn new(cmd_tx: Sender<ShellIpcCmd>) -> Self {
        Self { cmd_tx }
    }
}

#[interface(name = "org.axis.Shell")]
impl ShellIpcServer {
    /// Öffnet oder schließt den App-Launcher
    async fn toggle_launcher(&self) {
        let _ = self.cmd_tx.send(ShellIpcCmd::ToggleLauncher).await;
    }

    /// Öffnet oder schließt die Quick Settings
    async fn toggle_quick_settings(&self) {
        let _ = self.cmd_tx.send(ShellIpcCmd::ToggleQuickSettings).await;
    }

    /// Öffnet oder schließt die Workspace-Übersicht
    async fn toggle_workspaces(&self) {
        let _ = self.cmd_tx.send(ShellIpcCmd::ToggleWorkspaces).await;
    }

    /// Schließt alle aktiven Popups
    async fn close_all(&self) {
        let _ = self.cmd_tx.send(ShellIpcCmd::CloseAll).await;
    }
    
    /// Gibt die aktuelle Version der Shell zurück
    #[zbus(property)]
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }
}
