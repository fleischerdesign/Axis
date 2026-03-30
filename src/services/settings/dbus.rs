use async_channel::Sender;
use log::info;
use std::sync::{Arc, Mutex};
use zbus::interface;

use super::config::{AxisConfig, BarConfig, AppearanceConfig, NightlightConfig,
                    ContinuityConfig, ServicesConfig, ShortcutsConfig};
use super::SettingsCmd;

pub struct SettingsDbusServer {
    cmd_tx: Sender<SettingsCmd>,
    config: Arc<Mutex<AxisConfig>>,
}

impl SettingsDbusServer {
    pub fn new(cmd_tx: Sender<SettingsCmd>, config: Arc<Mutex<AxisConfig>>) -> Self {
        Self { cmd_tx, config }
    }
}

#[interface(name = "org.axis.Shell.Settings")]
impl SettingsDbusServer {
    // ── Full config (for initial load) ──────────────────────────────────

    async fn get_all_settings(&self) -> String {
        serde_json::to_string(&*self.config.lock().unwrap()).unwrap_or_default()
    }

    // ── Bar ─────────────────────────────────────────────────────────────

    async fn get_bar(&self) -> String {
        serde_json::to_string(&self.config.lock().unwrap().bar).unwrap_or_default()
    }

    async fn set_bar(&self, json: &str) -> bool {
        match serde_json::from_str::<BarConfig>(json) {
            Ok(cfg) => {
                let _ = self.cmd_tx.try_send(SettingsCmd::UpdateBar(cfg));
                true
            }
            Err(e) => {
                log::warn!("[settings-dbus] Invalid bar config: {e}");
                false
            }
        }
    }

    // ── Appearance ──────────────────────────────────────────────────────

    async fn get_appearance(&self) -> String {
        serde_json::to_string(&self.config.lock().unwrap().appearance).unwrap_or_default()
    }

    async fn set_appearance(&self, json: &str) -> bool {
        match serde_json::from_str::<AppearanceConfig>(json) {
            Ok(cfg) => {
                let _ = self.cmd_tx.try_send(SettingsCmd::UpdateAppearance(cfg));
                true
            }
            Err(e) => {
                log::warn!("[settings-dbus] Invalid appearance config: {e}");
                false
            }
        }
    }

    // ── Nightlight ──────────────────────────────────────────────────────

    async fn get_nightlight(&self) -> String {
        serde_json::to_string(&self.config.lock().unwrap().nightlight).unwrap_or_default()
    }

    async fn set_nightlight(&self, json: &str) -> bool {
        match serde_json::from_str::<NightlightConfig>(json) {
            Ok(cfg) => {
                let _ = self.cmd_tx.try_send(SettingsCmd::UpdateNightlight(cfg));
                true
            }
            Err(e) => {
                log::warn!("[settings-dbus] Invalid nightlight config: {e}");
                false
            }
        }
    }

    // ── Continuity ──────────────────────────────────────────────────────

    async fn get_continuity(&self) -> String {
        serde_json::to_string(&self.config.lock().unwrap().continuity).unwrap_or_default()
    }

    async fn set_continuity(&self, json: &str) -> bool {
        match serde_json::from_str::<ContinuityConfig>(json) {
            Ok(cfg) => {
                let _ = self.cmd_tx.try_send(SettingsCmd::UpdateContinuity(cfg));
                true
            }
            Err(e) => {
                log::warn!("[settings-dbus] Invalid continuity config: {e}");
                false
            }
        }
    }

    // ── Services ────────────────────────────────────────────────────────

    async fn get_services(&self) -> String {
        serde_json::to_string(&self.config.lock().unwrap().services).unwrap_or_default()
    }

    async fn set_services(&self, json: &str) -> bool {
        match serde_json::from_str::<ServicesConfig>(json) {
            Ok(cfg) => {
                let _ = self.cmd_tx.try_send(SettingsCmd::UpdateServices(cfg));
                true
            }
            Err(e) => {
                log::warn!("[settings-dbus] Invalid services config: {e}");
                false
            }
        }
    }

    // ── Shortcuts ───────────────────────────────────────────────────────

    async fn get_shortcuts(&self) -> String {
        serde_json::to_string(&self.config.lock().unwrap().shortcuts).unwrap_or_default()
    }

    async fn set_shortcuts(&self, json: &str) -> bool {
        match serde_json::from_str::<ShortcutsConfig>(json) {
            Ok(cfg) => {
                let _ = self.cmd_tx.try_send(SettingsCmd::UpdateShortcuts(cfg));
                true
            }
            Err(e) => {
                log::warn!("[settings-dbus] Invalid shortcuts config: {e}");
                false
            }
        }
    }

    // ── Open Settings App ───────────────────────────────────────────────

    async fn open_settings(&self) {
        info!("[settings-dbus] Opening settings app");
        let _ = std::process::Command::new("axis-settings")
            .spawn()
            .map_err(|e| log::warn!("[settings-dbus] Failed to launch axis-settings: {e}"));
    }

    // ── Version ─────────────────────────────────────────────────────────

    #[zbus(property)]
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    /// Signal emitted when settings change. Args: (section_key, json_payload)
    #[zbus(signal)]
    pub async fn settings_changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        section: &str,
        json: &str,
    ) -> zbus::Result<()>;
}
