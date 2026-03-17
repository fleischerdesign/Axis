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
use crate::services::launcher::LauncherService;
use crate::services::launcher::providers::apps::AppProvider;
use crate::store::ServiceStore;
use crate::widgets::{Bar, QuickSettingsPopup, WorkspacePopup, LauncherPopup};
use gtk4::prelude::*;
use gtk4::glib;

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

    // Launcher Service
    let (launcher_tx, launcher_rx) = async_channel::unbounded();
    let launcher_store = ServiceStore::new_manual(Default::default());
    let launcher_service = LauncherService::new(launcher_store.store.clone());
    
    // Provider registrieren (später mehr!)
    let launcher_service_init = launcher_service;
    glib::spawn_future_local(async move {
        launcher_service_init.add_provider(Box::new(AppProvider::default()));
        launcher_service_init.start(launcher_rx);
    });


    // --- STORES BAUEN (auf dem GTK-Main-Thread) ---
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

        launcher: launcher_store,
        launcher_tx,

        power: ServiceStore::new(power_rx, Default::default()),

        niri: ServiceStore::new(niri_rx, Default::default()),
        clock: ServiceStore::new(clock_rx, chrono::Local::now()),
    };

    // --- WIDGETS INITIALISIEREN ---
    let bar = Bar::new(app, ctx.clone());
    let ws_popup = WorkspacePopup::new(app, ctx.clone());
    let qs_popup = QuickSettingsPopup::new(app, &bar.vol_icon, ctx.clone());
    let launcher_popup = LauncherPopup::new(app, ctx.clone());

    // --- INTERAKTION ---
    let ws_is_open = ws_popup.is_open.clone();
    let qs_is_open = qs_popup.is_open.clone();
    let launcher_is_open = launcher_popup.is_open.clone();

    let popup_open = bar.popup_open.clone();
    let ws_is_open_ref = ws_is_open.clone();
    let qs_is_open_ref = qs_is_open.clone();
    let launcher_is_open_ref = launcher_is_open.clone();

    // Helper für Bar-Zustand
    let update_bar_popup_state = move || {
        *popup_open.borrow_mut() = *ws_is_open_ref.borrow() 
            || *qs_is_open_ref.borrow() 
            || *launcher_is_open_ref.borrow();
    };

    let update_bar_ws = update_bar_popup_state.clone();
    let ws_click = gtk4::GestureClick::new();
    ws_click.connect_pressed(move |_, _, _, _| {
        ws_popup.toggle();
        update_bar_ws();
    });
    bar.center_island.add_controller(ws_click);

    let update_bar_qs = update_bar_popup_state.clone();
    let qs_click = gtk4::GestureClick::new();
    qs_click.connect_pressed(move |_, _, _, _| {
        qs_popup.toggle();
        update_bar_qs();
    });
    bar.status_island.add_controller(qs_click);

    let update_bar_launcher = update_bar_popup_state.clone();
    let launcher_click = gtk4::GestureClick::new();
    launcher_click.connect_pressed(move |_, _, _, _| {
        launcher_popup.toggle();
        update_bar_launcher();
    });
    bar.launcher_island.add_controller(launcher_click);

    bar.window.present();
}
