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
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;

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
    
    // Provider registrieren
    let launcher_service_init = launcher_service;
    glib::spawn_future_local(async move {
        launcher_service_init.add_provider(Arc::new(AppProvider::default()));
        launcher_service_init.start(launcher_rx);
    });


    // --- STORES BAUEN ---
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

    // --- INTERAKTION ---
    let ws_is_open = ws_popup.is_open.clone();
    let qs_is_open = qs_popup.is_open.clone();
    let popup_open = bar.popup_open.clone();
    let bar_ref = bar.clone();

    // Wir brauchen einen Weg, den Launcher-State erst nach der Erstellung des Popups zu kennen.
    // Wir nutzen eine RefCell für die Launcher-Zustands-Referenz.
    let launcher_is_open_ptr: Rc<RefCell<Option<Rc<RefCell<bool>>>>> = Rc::new(RefCell::new(None));
    let launcher_ptr_cb = launcher_is_open_ptr.clone();

    let update_bar_popup_state = move || {
        let ws_open = *ws_is_open.borrow();
        let qs_open = *qs_is_open.borrow();
        let l_open = launcher_ptr_cb.borrow().as_ref()
            .map(|ptr| *ptr.borrow())
            .unwrap_or(false);

        *popup_open.borrow_mut() = ws_open || qs_open || l_open;
        bar_ref.check_auto_hide();
    };

    let update_cb = update_bar_popup_state.clone();
    let launcher_popup = LauncherPopup::new(app, ctx.clone(), move || {
        update_cb();
    });
    
    // Jetzt binden wir den echten State des Launchers ein
    *launcher_is_open_ptr.borrow_mut() = Some(launcher_popup.is_open.clone());

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
