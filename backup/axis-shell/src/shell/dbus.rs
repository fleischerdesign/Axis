use std::sync::{Arc, Mutex};
use std::rc::Rc;
use gtk4::glib;
use log::{info, error};
use zbus::connection::Builder;

use crate::app_context::AppContext;
use crate::shell::ShellController;
use crate::widgets::lock_screen::LockScreen;
use crate::services::ipc::IpcService;
use crate::services::ipc::server::ShellIpcCmd;
use axis_core::services::settings::dbus::SettingsDbusServer;
use axis_core::services::continuity::dbus::{ContinuityDbusServer, build_snapshot};
use axis_core::services::bluetooth::dbus::{BluetoothDbusServer, build_snapshot as build_bt_snapshot};
use axis_core::services::network::dbus::{NetworkDbusServer, build_snapshot as build_net_snapshot};

pub fn spawn_dbus_host(
    ctx: &AppContext,
    shell_ctrl: Rc<ShellController>,
    lock_screen: Rc<LockScreen>,
) {
    // 1. Prepare Servers
    let (ipc_server, ipc_rx) = IpcService::create_server();
    
    // ── Settings Cache + Notification ────────────────────────────────────
    let settings_config = Arc::new(Mutex::new(ctx.settings.store.get().config));
    let (settings_notify_tx, settings_notify_rx) = async_channel::unbounded::<()>();

    let cache = settings_config.clone();
    ctx.settings.store.subscribe(move |data| {
        *cache.lock().unwrap() = data.config.clone();
        let _ = settings_notify_tx.try_send(());
    });

    let settings_tx = ctx.settings.tx.clone();
    let settings_server = SettingsDbusServer::new(
        settings_tx,
        settings_config.clone(),
    );

    // ── Continuity State (watch channel, no Mutex) ─────────────────────
    let initial_snapshot = build_snapshot(&ctx.continuity.store.get());
    let (continuity_state_tx, continuity_state_rx) = tokio::sync::watch::channel(initial_snapshot);

    ctx.continuity.store.subscribe(move |data| {
        let _ = continuity_state_tx.send(build_snapshot(data));
    });

    let continuity_tx = ctx.continuity.tx.clone();
    let continuity_server = ContinuityDbusServer::new(continuity_tx, continuity_state_rx.clone());

    // ── Bluetooth State ────────────────────────────────────────────────────
    let initial_bt_snapshot = build_bt_snapshot(&ctx.bluetooth.store.get());
    let (bluetooth_state_tx, bluetooth_state_rx) = tokio::sync::watch::channel(initial_bt_snapshot);

    ctx.bluetooth.store.subscribe(move |data| {
        let _ = bluetooth_state_tx.send(build_bt_snapshot(data));
    });

    let bluetooth_tx = ctx.bluetooth.tx.clone();
    let bluetooth_server = BluetoothDbusServer::new(bluetooth_tx, bluetooth_state_rx.clone());

    // ── Network State ──────────────────────────────────────────────────────
    let initial_net_snapshot = build_net_snapshot(&ctx.network.store.get());
    let (network_state_tx, network_state_rx) = tokio::sync::watch::channel(initial_net_snapshot);

    ctx.network.store.subscribe(move |data| {
        let _ = network_state_tx.send(build_net_snapshot(data));
    });

    let network_tx = ctx.network.tx.clone();
    let network_server = NetworkDbusServer::new(network_tx, network_state_rx.clone());

    // 2. Setup IPC Command Loop (GTK Thread)
    let shell_ipc = shell_ctrl.clone();
    let ipc_lock = lock_screen.clone();
    glib::spawn_future_local(async move {
        while let Ok(cmd) = ipc_rx.recv().await {
            match cmd {
                ShellIpcCmd::ToggleLauncher => shell_ipc.toggle("launcher"),
                ShellIpcCmd::ToggleQuickSettings => shell_ipc.toggle("qs"),
                ShellIpcCmd::ToggleWorkspaces => shell_ipc.toggle("ws"),
                ShellIpcCmd::CloseAll => shell_ipc.close_all(),
                ShellIpcCmd::Lock => ipc_lock.lock_session(),
            }
        }
    });

    // 3. Register on D-Bus (Tokio Thread)
    tokio::spawn(async move {
        let conn_res = async {
            let builder = Builder::session()?;
            let builder = builder.name("org.axis.Shell")?;
            let builder = builder.serve_at("/org/axis/Shell", ipc_server)?;
            let builder = builder.serve_at("/org/axis/Shell/Settings", settings_server)?;
            let builder = builder.serve_at("/org/axis/Shell/Continuity", continuity_server)?;
            let builder = builder.serve_at("/org/axis/Shell/Bluetooth", bluetooth_server)?;
            let builder = builder.serve_at("/org/axis/Shell/Network", network_server)?;
            builder.build().await
        }.await;

        match conn_res {
            Ok(conn) => {
                info!("[dbus-host] D-Bus name 'org.axis.Shell' registered with multiple interfaces");

                // Settings signal loop
                let settings_loop = async {
                    while let Ok(()) = settings_notify_rx.recv().await {
                        let json = serde_json::to_string(&*settings_config.lock().unwrap())
                            .unwrap_or_default();
                        
                        let iface_res = conn
                            .object_server()
                            .interface::<_, SettingsDbusServer>("/org/axis/Shell/Settings")
                            .await;

                        if let Ok(iface) = iface_res {
                            let _ = SettingsDbusServer::settings_changed(
                                iface.signal_emitter(),
                                "all",
                                &json,
                            )
                            .await;
                        }
                    }
                };

                // Continuity signal loop
                let mut continuity_rx = continuity_state_rx;
                let continuity_loop = async {
                    loop {
                        if continuity_rx.changed().await.is_err() {
                            break;
                        }
                        let json = serde_json::to_string(&*continuity_rx.borrow_and_update())
                            .unwrap_or_default();
                        drop(continuity_rx.borrow_and_update());

                        let iface_res = conn
                            .object_server()
                            .interface::<_, ContinuityDbusServer>("/org/axis/Shell/Continuity")
                            .await;

                        if let Ok(iface) = iface_res {
                            let _ = ContinuityDbusServer::state_changed(
                                iface.signal_emitter(),
                                &json,
                            )
                            .await;
                        }
                    }
                };

                // Bluetooth signal loop
                let mut bluetooth_rx = bluetooth_state_rx;
                let bluetooth_loop = async {
                    loop {
                        if bluetooth_rx.changed().await.is_err() {
                            break;
                        }
                        let json = serde_json::to_string(&*bluetooth_rx.borrow_and_update())
                            .unwrap_or_default();
                        drop(bluetooth_rx.borrow_and_update());

                        let iface_res = conn
                            .object_server()
                            .interface::<_, BluetoothDbusServer>("/org/axis/Shell/Bluetooth")
                            .await;

                        if let Ok(iface) = iface_res {
                            let _ = BluetoothDbusServer::state_changed(
                                iface.signal_emitter(),
                                &json,
                            )
                            .await;
                        }
                    }
                };

                // Network signal loop
                let mut network_rx = network_state_rx;
                let network_loop = async {
                    loop {
                        if network_rx.changed().await.is_err() {
                            break;
                        }
                        let json = serde_json::to_string(&*network_rx.borrow_and_update())
                            .unwrap_or_default();
                        drop(network_rx.borrow_and_update());

                        let iface_res = conn
                            .object_server()
                            .interface::<_, NetworkDbusServer>("/org/axis/Shell/Network")
                            .await;

                        if let Ok(iface) = iface_res {
                            let _ = NetworkDbusServer::state_changed(
                                iface.signal_emitter(),
                                &json,
                            )
                            .await;
                        }
                    }
                };

                // Run all signal loops concurrently
                futures_util::join!(settings_loop, continuity_loop, bluetooth_loop, network_loop);
            }
            Err(e) => error!("[dbus-host] Failed to register D-Bus host: {:?}", e),
        }
    });
}
