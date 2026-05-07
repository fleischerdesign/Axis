pub mod composition;
pub mod config;
pub mod i18n;
pub mod presentation;
pub mod services;
pub mod utils;
pub mod widgets;

use axis_domain::models::appearance::{AccentColor, ColorScheme};
use clap::Parser;

#[derive(Parser)]
#[command(name = "axis-shell", about = "Axis Desktop Shell")]
pub struct Cli {
    #[arg(long)]
    pub wallpaper: Option<String>,
    #[arg(long)]
    pub locked: bool,
    #[arg(long)]
    pub accent: Option<String>,
    #[arg(long, value_name = "dark|light|system")]
    pub mode: Option<String>,
    #[arg(long)]
    pub font: Option<String>,
}

pub fn parse_accent(s: &str) -> AccentColor {
    match s.to_lowercase().as_str() {
        "blue" => AccentColor::Blue,
        "teal" => AccentColor::Teal,
        "green" => AccentColor::Green,
        "yellow" => AccentColor::Yellow,
        "orange" => AccentColor::Orange,
        "red" => AccentColor::Red,
        "pink" => AccentColor::Pink,
        "purple" => AccentColor::Purple,
        "auto" => AccentColor::Auto,
        _ => AccentColor::Custom(s.to_string()),
    }
}

pub fn parse_color_scheme(s: &str) -> Option<ColorScheme> {
    match s.to_lowercase().as_str() {
        "dark" => Some(ColorScheme::Dark),
        "light" => Some(ColorScheme::Light),
        "system" => Some(ColorScheme::System),
        _ => None,
    }
}

pub fn setup_logger() -> Result<(), fern::InitError> {
    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info);
    if let Ok(lvl) = std::env::var("RUST_LOG")
        && let Ok(parsed) = lvl.parse()
    {
        dispatch = dispatch.level(parsed);
    }
    dispatch.chain(std::io::stdout()).apply()?;
    Ok(())
}
