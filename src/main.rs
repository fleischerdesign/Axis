mod app_context;
mod services;
mod store;
mod widgets;


use crate::app_context::AppContext;
use crate::services::audio::AudioService;
use crate::services::backlight::BacklightService;
use crate::services::bluetooth::BluetoothService;
use crate::services::clock::ClockService;
use crate::services::nightlight::NightlightService;
use crate::services::network::NetworkService;
use crate::services::niri::NiriService;
use crate::services::power::PowerService;
use crate::store::ServiceStore;
use crate::widgets::{Bar, QuickSettingsPopup, WorkspacePopup};
use gtk4::prelude::*;

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

    // --- SERVICES STARTEN ---
    // Jeder Service läuft in einem eigenen Thread und sendet Daten über einen Channel.
    let (network_rx, network_tx) = NetworkService::spawn();
    let (bluetooth_rx, bluetooth_tx) = BluetoothService::spawn();
    let (audio_rx, audio_tx) = AudioService::spawn();
    let (backlight_rx, backlight_tx) = BacklightService::spawn();
    let backlight_initial = BacklightService::read_initial();
    let (nightlight_rx, nightlight_tx) = NightlightService::spawn();
    let nightlight_initial = NightlightService::read_initial();
    let power_rx = PowerService::spawn();
    let niri_rx = NiriService::spawn();
    let clock_rx = ClockService::spawn();


    // --- STORES BAUEN (auf dem GTK-Main-Thread) ---
    // Ab hier kein direktes Channel-Handling mehr in den Widgets.
    // Stores empfangen, cachen und broadcasten an alle Subscriber.
    let ctx = AppContext {
        network: ServiceStore::new(network_rx, Default::default()),
        network_tx,

        bluetooth: ServiceStore::new(bluetooth_rx, Default::default()),
        bluetooth_tx,

        audio: ServiceStore::new(audio_rx, Default::default()),
        audio_tx,

        backlight: ServiceStore::new(backlight_rx, backlight_initial),
        backlight_tx,

        nightlight: ServiceStore::new(nightlight_rx, nightlight_initial),
        nightlight_tx,

        power: ServiceStore::new(power_rx, Default::default()),

        niri: ServiceStore::new(niri_rx, Default::default()),
        clock: ServiceStore::new(clock_rx, chrono::Local::now()),
    };

    // --- WIDGETS INITIALISIEREN ---
    let bar = Bar::new(app, ctx.clone());
    let ws_popup = WorkspacePopup::new(app, ctx.clone());
    let qs_popup = QuickSettingsPopup::new(app, &bar.vol_icon, ctx.clone());

    // --- INTERAKTION ---
    let ws_click = gtk4::GestureClick::new();
    ws_click.connect_pressed(move |_, _, _, _| {
        ws_popup.toggle();
    });
    bar.center_island.add_controller(ws_click);

    let qs_click = gtk4::GestureClick::new();
    qs_click.connect_pressed(move |_, _, _, _| {
        qs_popup.toggle();
    });
    bar.status_island.add_controller(qs_click);

    bar.window.present();
}
