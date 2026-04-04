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

        thread::spawn(move || {
            let mut config = ConfigManager::load();
            info!("[settings] Config loaded from {}", ConfigManager::config_path().display());

            let initial = SettingsData { config: config.clone() };
            let _ = data_tx.send_blocking(initial);

            // Start file watcher (sends Reload on external changes)
            watcher::ConfigWatcher::spawn(watcher_tx, suppress_reload);

            loop {
                match cmd_rx.recv_blocking() {
                    Ok(cmd) => {
                        let changed = Self::apply_cmd(cmd, &mut config);
                        if changed {
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
            SettingsCmd::UpdateBar(c) => update_if_changed(&mut config.bar, c),
            SettingsCmd::UpdateAppearance(c) => update_if_changed(&mut config.appearance, c),
            SettingsCmd::UpdateNightlight(c) => update_if_changed(&mut config.nightlight, c),
            SettingsCmd::UpdateContinuity(c) => update_if_changed(&mut config.continuity, c),
            SettingsCmd::UpdateServices(c) => update_if_changed(&mut config.services, c),
            SettingsCmd::UpdateShortcuts(c) => update_if_changed(&mut config.shortcuts, c),
            SettingsCmd::UpdatePartial(apply_fn) => {
                let old = config.clone();
                apply_fn(config);
                *config != old
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

fn update_if_changed<T: PartialEq>(field: &mut T, new: T) -> bool {
    if *field != new {
        *field = new;
        true
    } else {
        false
    }
}
