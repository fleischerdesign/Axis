use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Config Section Trait (OCP) ──────────────────────────────────────────────

pub trait ConfigSection:
    Default + Serialize + for<'de> Deserialize<'de> + Clone + PartialEq + Send + 'static
{
    /// Unique key for D-Bus property names, e.g. "Bar", "Appearance"
    const SECTION_KEY: &'static str;
}

// ── Helper Functions for serde defaults ──────────────────────────────────────

fn default_true() -> bool {
    true
}

fn default_temp_day() -> u32 {
    6500
}
fn default_temp_night() -> u32 {
    4500
}
fn default_sunrise() -> String {
    "07:00".into()
}
fn default_sunset() -> String {
    "20:00".into()
}
fn default_shortcut_launcher() -> String {
    "<Super>space".into()
}
fn default_shortcut_qs() -> String {
    "<Super>s".into()
}
fn default_shortcut_ws() -> String {
    "<Super>w".into()
}
fn default_shortcut_lock() -> String {
    "<Super>l".into()
}

// ── Bar Config ──────────────────────────────────────────────────────────────

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
impl ConfigSection for BarConfig {
    const SECTION_KEY: &'static str = "Bar";
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum BarPosition {
    Top,
    #[default]
    Bottom,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum BarLayer {
    #[default]
    Top,
    Bottom,
    Overlay,
}

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
        Self {
            launcher: true,
            clock: true,
            status: true,
            workspace: true,
        }
    }
}

// ── Appearance Config ───────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default)]
    pub wallpaper: Option<String>,
    #[serde(default)]
    pub accent_color: AccentColor,
    #[serde(default)]
    pub font: Option<String>,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            wallpaper: None,
            accent_color: AccentColor::default(),
            font: None,
        }
    }
}
impl ConfigSection for AppearanceConfig {
    const SECTION_KEY: &'static str = "Appearance";
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AccentColor {
    #[default]
    Blue,
    Teal,
    Green,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
    Auto,
}

impl AccentColor {
    pub fn hex_value(&self) -> &'static str {
        match self {
            Self::Blue   => "#3584e4",
            Self::Teal   => "#33b5a0",
            Self::Green  => "#3ec35a",
            Self::Yellow => "#f5c211",
            Self::Orange => "#ed5b00",
            Self::Red    => "#e53935",
            Self::Pink   => "#e45b9c",
            Self::Purple => "#9141ac",
            Self::Auto   => "#3584e4",
        }
    }

    pub fn all_presets() -> &'static [AccentColor] {
        &[
            Self::Blue, Self::Teal, Self::Green, Self::Yellow,
            Self::Orange, Self::Red, Self::Pink, Self::Purple, Self::Auto,
        ]
    }
}

// ── Nightlight Config ───────────────────────────────────────────────────────

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
            enabled: false,
            temp_day: 6500,
            temp_night: 4500,
            sunrise: "07:00".into(),
            sunset: "20:00".into(),
            latitude: String::new(),
            longitude: String::new(),
        }
    }
}
impl ConfigSection for NightlightConfig {
    const SECTION_KEY: &'static str = "Nightlight";
}

// ── Continuity Config ───────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContinuityConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub peer_configs: Vec<PeerPersistedConfig>,
}
impl ConfigSection for ContinuityConfig {
    const SECTION_KEY: &'static str = "Continuity";
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PeerPersistedConfig {
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

// ── Services Config ─────────────────────────────────────────────────────────

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
impl ConfigSection for ServicesConfig {
    const SECTION_KEY: &'static str = "Services";
}

// ── Shortcuts Config ────────────────────────────────────────────────────────

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
impl ConfigSection for ShortcutsConfig {
    const SECTION_KEY: &'static str = "Shortcuts";
}

// ── AxisConfig (Aggregate) ──────────────────────────────────────────────────

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

// ── ConfigManager (SRP: File I/O only) ──────────────────────────────────────

pub struct ConfigManager;

impl ConfigManager {
    pub fn config_dir() -> PathBuf {
        dirs_fallback()
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> AxisConfig {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<AxisConfig>(&contents) {
                Ok(config) => config,
                Err(e) => {
                    log::warn!("[settings] Failed to parse {}: {e}", path.display());
                    AxisConfig::default()
                }
            },
            Err(_) => AxisConfig::default(),
        }
    }

    pub fn save(config: &AxisConfig) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::warn!("[settings] Failed to create config dir: {e}");
                return;
            }
        }

        let json = match serde_json::to_string_pretty(config) {
            Ok(j) => j,
            Err(e) => {
                log::warn!("[settings] Failed to serialize config: {e}");
                return;
            }
        };

        // Atomic write: tmp + rename
        let tmp_path = path.with_extension("tmp");
        if let Err(e) = std::fs::write(&tmp_path, json) {
            log::warn!("[settings] Failed to write config tmp: {e}");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp_path, &path) {
            log::warn!("[settings] Failed to rename config: {e}");
        }
    }

    pub fn exists() -> bool {
        Self::config_path().exists()
    }
}

fn dirs_fallback() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".config")
        })
        .join("axis")
}
