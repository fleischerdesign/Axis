//! Config types mirrored from the shell's services::settings::config.
//! These must stay in sync with the shell config for serde compatibility.
//! Both sides serialize/deserialize the same JSON format.

use serde::{Deserialize, Serialize};

// ── Defaults ────────────────────────────────────────────────────────────────

fn default_true() -> bool { true }
fn default_opacity() -> f64 { 1.0 }
fn default_corner_radius() -> u32 { 10 }
fn default_temp_day() -> u32 { 6500 }
fn default_temp_night() -> u32 { 4500 }
fn default_sunrise() -> String { "07:00".into() }
fn default_sunset() -> String { "20:00".into() }
fn default_shortcut_launcher() -> String { "<Super>space".into() }
fn default_shortcut_qs() -> String { "<Super>s".into() }
fn default_shortcut_ws() -> String { "<Super>w".into() }
fn default_shortcut_lock() -> String { "<Super>l".into() }

// ── Bar ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AxisConfig {
    #[serde(default)]
    pub bar: BarConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub nightlight: NightlightConfig,
    #[serde(default)]
    pub continuity: ContinuityConfig,
    #[serde(default)]
    pub services: ServicesConfig,
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BarConfig {
    #[serde(default)]
    pub position: BarPosition,
    #[serde(default = "default_true")]
    pub autohide: bool,
    #[serde(default)]
    pub layer: BarLayer,
    #[serde(default)]
    pub islands: IslandVisibility,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum BarPosition { Top, #[default] Bottom }

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum BarLayer { #[default] Top, Bottom, Overlay }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IslandVisibility {
    #[serde(default = "default_true")]
    pub launcher: bool,
    #[serde(default = "default_true")]
    pub clock: bool,
    #[serde(default = "default_true")]
    pub status: bool,
    #[serde(default = "default_true")]
    pub workspace: bool,
}
impl Default for IslandVisibility {
    fn default() -> Self {
        Self { launcher: true, clock: true, status: true, workspace: true }
    }
}

// ── Appearance ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_opacity")]
    pub bar_opacity: f64,
    #[serde(default = "default_corner_radius")]
    pub corner_radius: u32,
}
impl Default for AppearanceConfig {
    fn default() -> Self {
        Self { theme: Theme::default(), bar_opacity: 1.0, corner_radius: 10 }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Theme { Light, Dark, #[default] System }

// ── Nightlight ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NightlightConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_temp_day")]
    pub temp_day: u32,
    #[serde(default = "default_temp_night")]
    pub temp_night: u32,
    #[serde(default = "default_sunrise")]
    pub sunrise: String,
    #[serde(default = "default_sunset")]
    pub sunset: String,
    #[serde(default)]
    pub latitude: String,
    #[serde(default)]
    pub longitude: String,
}
impl Default for NightlightConfig {
    fn default() -> Self {
        Self {
            enabled: false, temp_day: 6500, temp_night: 4500,
            sunrise: "07:00".into(), sunset: "20:00".into(),
            latitude: String::new(), longitude: String::new(),
        }
    }
}

// ── Continuity ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContinuityConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub peer_configs: Vec<PeerConfig>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PeerConfig {
    pub device_id: String,
    pub device_name: String,
    #[serde(default = "default_true")]
    pub clipboard: bool,
    #[serde(default)]
    pub audio: bool,
    #[serde(default)]
    pub drag_drop: bool,
    #[serde(default)]
    pub arrangement_x: i32,
    #[serde(default)]
    pub arrangement_y: i32,
}

// ── Services ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServicesConfig {
    #[serde(default = "default_true")]
    pub bluetooth_enabled: bool,
    #[serde(default)]
    pub airplane_enabled: bool,
    #[serde(default)]
    pub dnd_enabled: bool,
}
impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            bluetooth_enabled: true,
            airplane_enabled: false,
            dnd_enabled: false,
        }
    }
}

// ── Shortcuts ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    #[serde(default = "default_shortcut_launcher")]
    pub launcher: String,
    #[serde(default = "default_shortcut_qs")]
    pub quick_settings: String,
    #[serde(default = "default_shortcut_ws")]
    pub workspaces: String,
    #[serde(default = "default_shortcut_lock")]
    pub lock: String,
}
impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            launcher: "<Super>space".into(),
            quick_settings: "<Super>s".into(),
            workspaces: "<Super>w".into(),
            lock: "<Super>l".into(),
        }
    }
}
