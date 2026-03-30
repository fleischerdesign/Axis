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
    UpdateContinuityPartial(Box<dyn FnOnce(&mut ContinuityConfig) + Send>),
    UpdateServices(ServicesConfig),
    UpdateServicesPartial(Box<dyn FnOnce(&mut ServicesConfig) + Send>),
    UpdateShortcuts(ShortcutsConfig),
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

        thread::spawn(move || {
            let mut config = ConfigManager::load();
            info!("[settings] Config loaded from {}", ConfigManager::config_path().display());

            let initial = SettingsData { config: config.clone() };
            let _ = data_tx.send_blocking(initial);

            // Start file watcher (sends Reload on external changes)
            // Pass suppress flag so saves from SettingsService don't trigger reload
            watcher::ConfigWatcher::spawn(watcher_tx, suppress_reload);

            loop {
                match cmd_rx.recv_blocking() {
                    Ok(cmd) => {
                        let changed = Self::apply_cmd(cmd, &mut config);
                        if changed {
                            // Suppress the file watcher for this save
                            suppress_for_save.store(true, Ordering::SeqCst);
                            ConfigManager::save(&config);
                            let _ = data_tx.send_blocking(SettingsData { config: config.clone() });
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        (
            ServiceStore::new(data_rx, SettingsData { config: ConfigManager::load() }),
            cmd_tx,
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
            SettingsCmd::UpdateContinuityPartial(apply_fn) => {
                let old = config.continuity.clone();
                apply_fn(&mut config.continuity);
                config.continuity != old
            }
            SettingsCmd::UpdateServices(c) => {
                if config.services != c {
                    config.services = c;
                    true
                } else {
                    false
                }
            }
            SettingsCmd::UpdateServicesPartial(apply_fn) => {
                let old = config.services.clone();
                apply_fn(&mut config.services);
                config.services != old
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
