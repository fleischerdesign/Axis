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

impl Default for NightlightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            temp_day: 6500,
            temp_night: 4500,
            sunrise: "07:00".into(),
            sunset: "20:00".into(),
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
            launcher: true,
            clock: true,
            status: true,
            workspace: true,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContinuityConfig {
    #[serde(default)]
    pub enabled: bool,
}
