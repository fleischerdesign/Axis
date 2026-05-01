use axis_infrastructure::adapters::continuity::dbus::ContinuityDbusServer;
use axis_infrastructure::adapters::continuity::ContinuityCmd;
use axis_domain::models::continuity::ContinuityStatus;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::watch;
use zbus::connection;

struct ShellIface {
    on_toggle_launcher: Arc<dyn Fn() + Send + Sync>,
    on_lock: Arc<dyn Fn() + Send + Sync>,
}

#[zbus::interface(name = "org.axis.Shell")]
impl ShellIface {
    async fn toggle_launcher(&self) -> zbus::fdo::Result<()> {
        (self.on_toggle_launcher)();
        Ok(())
    }

    async fn lock(&self) -> zbus::fdo::Result<()> {
        (self.on_lock)();
        Ok(())
    }
}

pub async fn run_dbus_host(
    on_toggle_launcher: impl Fn() + Send + Sync + 'static,
    on_lock: impl Fn() + Send + Sync + 'static,
    continuity_cmd_tx: async_channel::Sender<ContinuityCmd>,
    continuity_status_rx: watch::Receiver<ContinuityStatus>,
) {
    let ipc_iface = ShellIface {
        on_toggle_launcher: Arc::new(on_toggle_launcher),
        on_lock: Arc::new(on_lock),
    };

    let (status_tx, status_rx) = watch::channel(ContinuityStatus::default());
    let cont_server = ContinuityDbusServer::new(continuity_cmd_tx, status_rx.clone());

    let conn = match connection::Builder::session() {
        Ok(b) => b,
        Err(e) => {
            error!("[dbus-host] Failed to create D-Bus builder: {e}");
            return;
        }
    };

    let conn = match conn.name("org.axis.Shell") {
        Ok(c) => c,
        Err(e) => {
            error!("[dbus-host] Failed to claim D-Bus name: {e}");
            return;
        }
    };

    let conn = match conn.serve_at("/org/axis/Shell", ipc_iface) {
        Ok(c) => c,
        Err(e) => {
            error!("[dbus-host] Failed to serve IPC interface: {e}");
            return;
        }
    };

    let conn = match conn.serve_at("/org/axis/Shell/Continuity", cont_server) {
        Ok(c) => c,
        Err(e) => {
            error!("[dbus-host] Failed to serve Continuity interface: {e}");
            return;
        }
    };

    let conn = match conn.build().await {
        Ok(c) => c,
        Err(e) => {
            error!("[dbus-host] Failed to build D-Bus connection: {e}");
            return;
        }
    };

    info!("[dbus-host] D-Bus server started on org.axis.Shell with IPC + Continuity");

    let mut status_rx = continuity_status_rx;
    let snapshot_loop = async {
        loop {
            if status_rx.changed().await.is_err() {
                break;
            }
            let status = status_rx.borrow_and_update().clone();
            let _ = status_tx.send(status.clone());

            let iface_res: Result<_, _> = conn
                .object_server()
                .interface::<&str, ContinuityDbusServer>("/org/axis/Shell/Continuity")
                .await;

            if let Ok(iface) = iface_res {
                let json = serde_json::to_string(&status).unwrap_or_default();
                let _ = ContinuityDbusServer::state_changed(
                    iface.signal_emitter(),
                    &json,
                ).await;
            }
        }
    };

    snapshot_loop.await;
}
