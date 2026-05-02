use serde::{Deserialize, Serialize};

use super::appearance::{AccentColor, ColorScheme};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AxisConfig {
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub dnd: DndConfig,
    #[serde(default)]
    pub nightlight: NightlightConfig,
    #[serde(default)]
    pub airplane: AirplaneConfig,
    #[serde(default)]
    pub bar: BarConfig,
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
    #[serde(default)]
    pub continuity: ContinuityConfig,
    #[serde(default)]
    pub idle: IdleConfig,
    #[serde(default)]
    pub idle_inhibit: IdleInhibitConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default)]
    pub wallpaper: Option<String>,
    #[serde(default)]
    pub accent_color: AccentColor,
    #[serde(default)]
    pub color_scheme: ColorScheme,
    #[serde(default)]
    pub font: Option<String>,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            wallpaper: None,
            accent_color: AccentColor::default(),
            color_scheme: ColorScheme::default(),
            font: None,
        }
    }
}

impl AppearanceConfig {
    pub fn is_default_accent(&self) -> bool {
        self.accent_color == AccentColor::default()
    }

    pub fn is_default_scheme(&self) -> bool {
        self.color_scheme == ColorScheme::default()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DndConfig {
    #[serde(default)]
    pub enabled: bool,
}

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
    pub auto_schedule: bool,
    #[serde(default)]
    pub latitude: String,
    #[serde(default)]
    pub longitude: String,
}

impl NightlightConfig {
    pub const DEFAULT_TEMP_DAY: u32 = 6500;
    pub const DEFAULT_TEMP_NIGHT: u32 = 4500;
    pub const DEFAULT_SUNRISE: &'static str = "07:00";
    pub const DEFAULT_SUNSET: &'static str = "20:00";
}

fn default_temp_day() -> u32 {
    NightlightConfig::DEFAULT_TEMP_DAY
}
fn default_temp_night() -> u32 {
    NightlightConfig::DEFAULT_TEMP_NIGHT
}
fn default_sunrise() -> String {
    NightlightConfig::DEFAULT_SUNRISE.into()
}
fn default_sunset() -> String {
    NightlightConfig::DEFAULT_SUNSET.into()
}

impl Default for NightlightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            temp_day: Self::DEFAULT_TEMP_DAY,
            temp_night: Self::DEFAULT_TEMP_NIGHT,
            sunrise: Self::DEFAULT_SUNRISE.into(),
            sunset: Self::DEFAULT_SUNSET.into(),
            auto_schedule: false,
            latitude: String::new(),
            longitude: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AirplaneConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BarConfig {
    #[serde(default)]
    pub position: BarPosition,
    #[serde(default = "default_true")]
    pub autohide: bool,
    #[serde(default)]
    pub islands: IslandVisibility,
    #[serde(default)]
    pub show_labels: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum BarPosition {
    Top,
    #[default]
    Bottom,
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
            launcher: default_true(),
            clock: default_true(),
            status: default_true(),
            workspace: default_true(),
        }
    }
}

fn default_true() -> bool {
    true
}

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

fn default_shortcut_launcher() -> String {
    ShortcutsConfig::DEFAULT_LAUNCHER.into()
}
fn default_shortcut_qs() -> String {
    ShortcutsConfig::DEFAULT_QUICK_SETTINGS.into()
}
fn default_shortcut_ws() -> String {
    ShortcutsConfig::DEFAULT_WORKSPACES.into()
}
fn default_shortcut_lock() -> String {
    ShortcutsConfig::DEFAULT_LOCK.into()
}

impl ShortcutsConfig {
    pub const DEFAULT_LAUNCHER: &'static str = "<Super>space";
    pub const DEFAULT_QUICK_SETTINGS: &'static str = "<Super>s";
    pub const DEFAULT_WORKSPACES: &'static str = "<Super>w";
    pub const DEFAULT_LOCK: &'static str = "<Super>l";
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            launcher: Self::DEFAULT_LAUNCHER.into(),
            quick_settings: Self::DEFAULT_QUICK_SETTINGS.into(),
            workspaces: Self::DEFAULT_WORKSPACES.into(),
            lock: Self::DEFAULT_LOCK.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContinuityConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdleInhibitConfig {
    #[serde(default)]
    pub enabled: bool,
}

impl Default for IdleInhibitConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdleConfig {
    #[serde(default)]
    pub lock_timeout_seconds: Option<u32>,
    #[serde(default)]
    pub blank_timeout_seconds: Option<u32>,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            lock_timeout_seconds: None,
            blank_timeout_seconds: None,
        }
    }
}
