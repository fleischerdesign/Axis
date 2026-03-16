mod services;
mod widgets;
mod app_context;

use gtk4::prelude::*;
use crate::widgets::{Bar, QuickSettingsPopup, WorkspacePopup};
use crate::app_context::AppContext;
use crate::services::network::NetworkService;
use crate::services::bluetooth::BluetoothService;
use crate::services::audio::AudioService;
use crate::services::power::PowerService;
use crate::services::niri::NiriService;
use crate::services::clock::ClockService;

#[tokio::main]
async fn main() {
    let application = libadwaita::Application::builder()
        .application_id("com.github.carp.shell")
        .build();

    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(app: &libadwaita::Application) {
    // CSS laden
    let provider = gtk4::CssProvider::new();
    provider.load_from_path("src/style.css");
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::PreferDark);

    // --- SERVICES STARTEN (Global) ---
    let (network_rx, network_tx) = NetworkService::spawn();
    let (bluetooth_rx, bluetooth_tx) = BluetoothService::spawn();
    let (audio_rx, audio_tx) = AudioService::spawn();
    let power_rx = PowerService::spawn();
    let niri_rx = NiriService::spawn();
    let clock_rx = ClockService::spawn();

    let ctx = AppContext {
        network_rx,
        network_tx,
        bluetooth_rx,
        bluetooth_tx,
        audio_rx,
        audio_tx,
        power_rx,
        niri_rx,
        clock_rx,
    };

    // --- KOMPONENTEN INITIALISIEREN ---
    let bar = Bar::new(app, ctx.clone());
    let ws_popup = WorkspacePopup::new(app, ctx.clone());
    let qs_popup = QuickSettingsPopup::new(app, &bar.vol_icon, ctx.clone());

    // --- INTERACTION ---
    let ws_click = gtk4::GestureClick::new();
    let ctx_ws = ctx.clone();
    ws_click.connect_pressed(move |_, _, _, _| { ws_popup.toggle(&ctx_ws); });
    bar.center_island.add_controller(ws_click);

    let qs_click = gtk4::GestureClick::new();
    qs_click.connect_pressed(move |_, _, _, _| { qs_popup.toggle(); });
    bar.status_island.add_controller(qs_click);

    bar.window.present();
}
