pub mod config;
pub mod dbus;
pub mod sync;
pub mod watcher;

pub use config::AxisConfig;
pub use config::ConfigManager;

use async_channel::{bounded, Sender};
use config::*;
use crate::store::ServiceStore;
use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use super::Service;

// ── Service Data ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct SettingsData {
    pub config: AxisConfig,
}

impl PartialEq for SettingsData {
    fn eq(&self, other: &Self) -> bool {
        self.config == other.config
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

pub enum SettingsCmd {
    UpdateBar(BarConfig),
    UpdateAppearance(AppearanceConfig),
    UpdateNightlight(NightlightConfig),
    UpdateContinuity(ContinuityConfig),
    UpdateServices(ServicesConfig),
    UpdateShortcuts(ShortcutsConfig),
    UpdatePartial(Box<dyn FnOnce(&mut AxisConfig) + Send>),
    Reload,
}

// ── Service ─────────────────────────────────────────────────────────────────

pub struct SettingsService;

impl Service for SettingsService {
    type Data = SettingsData;
    type Cmd = SettingsCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (data_tx, data_rx) = bounded(16);
        let (cmd_tx, cmd_rx) = bounded(32);

        let watcher_tx = cmd_tx.clone();
        let suppress_reload = Arc::new(AtomicBool::new(false));
        let suppress_for_save = suppress_reload.clone();

        // Shared config for D-Bus server and signal emission
        let settings_config = Arc::new(std::sync::Mutex::new(ConfigManager::load()));

        // cmd_tx is moved into the thread; return a fresh clone from outside
        let cmd_tx_out = cmd_tx.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for SettingsService");

            let mut config = ConfigManager::load();
            info!("[settings] Config loaded from {}", ConfigManager::config_path().display());

            let initial = SettingsData { config: config.clone() };
            let _ = data_tx.send_blocking(initial);

            // Sync initial config to shared state
            *settings_config.lock().unwrap() = config.clone();

            // Start file watcher (sends Reload on external changes)
            watcher::ConfigWatcher::spawn(watcher_tx, suppress_reload);

            // Spawn D-Bus server on the tokio runtime (runs on worker threads)
            let dbus_config = settings_config.clone();
            let dbus_cmd_tx = cmd_tx.clone();
            rt.spawn(async move {
                use zbus::connection::Builder;
                let server = dbus::SettingsDbusServer::new(dbus_cmd_tx, dbus_config);
                match Builder::session()?
                    .name("org.axis.Shell")?
                    .serve_at("/org/axis/Shell/Settings", server)?
                    .build()
                    .await
                {
                    Ok(_conn) => {
                        info!("[settings] D-Bus Interface 'org.axis.Shell.Settings' registered");
                        // Keep connection alive — runs forever
                        std::future::pending::<()>().await;
                    }
                    Err(e) => log::error!("[settings] Failed to register D-Bus interface: {:?}", e),
                }
                Ok::<(), zbus::Error>(())
            });

            // Main service loop: handle commands, save config, emit D-Bus signals
            loop {
                match cmd_rx.recv_blocking() {
                    Ok(cmd) => {
                        let changed = Self::apply_cmd(cmd, &mut config);
                        if changed {
                            suppress_for_save.store(true, Ordering::SeqCst);
                            ConfigManager::save(&config);
                            *settings_config.lock().unwrap() = config.clone();
                            let _ = data_tx.send_blocking(SettingsData { config: config.clone() });

                            // Emit D-Bus SettingsChanged signal via the runtime
                            let sc = settings_config.clone();
                            rt.block_on(async move {
                                let json = serde_json::to_string(&*sc.lock().unwrap())
                                    .unwrap_or_default();
                                // We need a connection to emit — get it from the object server
                                // Use a temporary proxy to find the registered interface
                                if let Ok(conn) = zbus::Connection::session().await {
                                    let iface = conn.object_server()
                                        .interface::<_, dbus::SettingsDbusServer>(
                                            "/org/axis/Shell/Settings",
                                        )
                                        .await;
                                    if let Ok(iface) = iface {
                                        let _ = dbus::SettingsDbusServer::settings_changed(
                                            iface.signal_emitter(),
                                            "all",
                                            &json,
                                        ).await;
                                    }
                                }
                            });
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        (
            ServiceStore::new(data_rx, SettingsData { config: ConfigManager::load() }),
            cmd_tx_out,
        )
    }
}

impl SettingsService {
    fn apply_cmd(cmd: SettingsCmd, config: &mut AxisConfig) -> bool {
        match cmd {
            SettingsCmd::UpdateBar(c) => {
                if config.bar != c {
                    config.bar = c;
                    true
                } else {
                    false
                }
            }
            SettingsCmd::UpdateAppearance(c) => {
                if config.appearance != c {
                    config.appearance = c;
                    true
                } else {
                    false
                }
            }
            SettingsCmd::UpdateNightlight(c) => {
                if config.nightlight != c {
                    config.nightlight = c;
                    true
                } else {
                    false
                }
            }
            SettingsCmd::UpdateContinuity(c) => {
                if config.continuity != c {
                    config.continuity = c;
                    true
                } else {
                    false
                }
            }
            SettingsCmd::UpdateServices(c) => {
                if config.services != c {
                    config.services = c;
                    true
                } else {
                    false
                }
            }
            SettingsCmd::UpdatePartial(apply_fn) => {
                let old = config.clone();
                apply_fn(config);
                *config != old
            }
            SettingsCmd::UpdateShortcuts(c) => {
                if config.shortcuts != c {
                    config.shortcuts = c;
                    true
                } else {
                    false
                }
            }
            SettingsCmd::Reload => {
                let new_config = ConfigManager::load();
                if *config != new_config {
                    *config = new_config;
                    true
                } else {
                    false
                }
            }
        }
    }
}
