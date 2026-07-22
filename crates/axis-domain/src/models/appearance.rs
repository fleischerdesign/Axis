use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    Custom(String),
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
        hex.starts_with('#')
            && (hex.len() == 7 || hex.len() == 4)
            && hex[1..].chars().all(|c| c.is_ascii_hexdigit())
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorScheme {
    #[default]
    Dark,
    Light,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_value_blue() {
        assert_eq!(AccentColor::Blue.hex_value(), "#3584e4");
    }

    #[test]
    fn hex_value_custom() {
        assert_eq!(AccentColor::Custom("#123abc".into()).hex_value(), "#123abc");
    }

    #[test]
    fn is_valid_hex_6_digit() {
        assert!(AccentColor::is_valid_hex("#123abc"));
        assert!(AccentColor::is_valid_hex("#ABCDEF"));
    }

    #[test]
    fn is_valid_hex_3_digit() {
        assert!(AccentColor::is_valid_hex("#fff"));
        assert!(AccentColor::is_valid_hex("#1a3"));
    }

    #[test]
    fn is_valid_hex_invalid() {
        assert!(!AccentColor::is_valid_hex("123abc"));
        assert!(!AccentColor::is_valid_hex("#12345"));
        assert!(!AccentColor::is_valid_hex("#12345g"));
        assert!(!AccentColor::is_valid_hex(""));
    }

    #[test]
    fn all_presets_count() {
        assert_eq!(AccentColor::all_presets().len(), 9);
    }

    #[test]
    fn color_scheme_default_is_dark() {
        assert_eq!(ColorScheme::default(), ColorScheme::Dark);
    }

    #[test]
    fn accent_color_serde_roundtrip() {
        let colors = vec![
            AccentColor::Blue,
            AccentColor::Teal,
            AccentColor::Custom("#ff8800".into()),
            AccentColor::Auto,
        ];
        for c in colors {
            let json = serde_json::to_string(&c).unwrap();
            let back: AccentColor = serde_json::from_str(&json).unwrap();
            assert_eq!(c, back);
        }
    }
}
