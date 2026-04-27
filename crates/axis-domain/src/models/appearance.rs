use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AccentColor {
    Blue,
    Teal,
    Green,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
    Auto,
    Custom(String),
}

impl Default for AccentColor {
    fn default() -> Self {
        Self::Blue
    }
}

impl AccentColor {
    pub fn hex_value(&self) -> Cow<'_, str> {
        match self {
            Self::Blue => "#3584e4",
            Self::Teal => "#33b5a0",
            Self::Green => "#3ec35a",
            Self::Yellow => "#f5c211",
            Self::Orange => "#ed5b00",
            Self::Red => "#e53935",
            Self::Pink => "#e45b9c",
            Self::Purple => "#9141ac",
            Self::Auto => "#3584e4",
            Self::Custom(hex) => hex.as_str(),
        }
        .into()
    }

    pub fn is_valid_hex(hex: &str) -> bool {
        hex.starts_with('#') && (hex.len() == 7 || hex.len() == 4) && hex[1..].chars().all(|c| c.is_ascii_hexdigit())
    }

    pub fn all_presets() -> &'static [AccentColor] {
        &[
            Self::Blue,
            Self::Teal,
            Self::Green,
            Self::Yellow,
            Self::Orange,
            Self::Red,
            Self::Pink,
            Self::Purple,
            Self::Auto,
        ]
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ColorScheme {
    #[default]
    Dark,
    Light,
    System,
}
