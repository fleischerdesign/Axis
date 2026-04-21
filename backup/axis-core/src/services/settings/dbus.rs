use async_channel::Sender;
use log::info;
use std::sync::{Arc, Mutex};
use zbus::interface;

use super::config::{AxisConfig, BarConfig, AppearanceConfig, NightlightConfig,
                    ContinuityConfig, ServicesConfig, ShortcutsConfig, ConfigSection};
use super::SettingsCmd;

macro_rules! dbus_section {
    ($config_ty:ty, $cmd:ident, $getter:ident, $setter:ident, $field:ident) => {
        async fn $getter(&self) -> String {
            serde_json::to_string(&self.config.lock().unwrap().$field).unwrap_or_default()
        }

        async fn $setter(&self, json: &str) -> bool {
            match serde_json::from_str::<$config_ty>(json) {
                Ok(cfg) => {
                    let _ = self.cmd_tx.try_send(SettingsCmd::$cmd(cfg));
                    true
                }
                Err(e) => {
                    log::warn!("[settings-dbus] Invalid {} config: {e}", <$config_ty as ConfigSection>::SECTION_KEY);
                    false
                }
            }
        }
    };
}

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

    // ── Sections (generated via macro) ──────────────────────────────────

    dbus_section!(BarConfig, UpdateBar, get_bar, set_bar, bar);
    dbus_section!(AppearanceConfig, UpdateAppearance, get_appearance, set_appearance, appearance);
    dbus_section!(NightlightConfig, UpdateNightlight, get_nightlight, set_nightlight, nightlight);
    dbus_section!(ContinuityConfig, UpdateContinuity, get_continuity, set_continuity, continuity);
    dbus_section!(ServicesConfig, UpdateServices, get_services, set_services, services);
    dbus_section!(ShortcutsConfig, UpdateShortcuts, get_shortcuts, set_shortcuts, shortcuts);

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
