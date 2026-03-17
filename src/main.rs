mod app_context;
mod services;
mod store;
mod widgets;
mod shell;

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
use crate::shell::ShellController;
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
    let ctx = setup_services();

    // --- WIDGETS & CONTROLLER ---
    let bar = Bar::new(app, ctx.clone());
    
    // Der Controller orchestriert alle Popups
    let bar_popup_state = bar.popup_open.clone();
    let bar_ref = bar.clone();
    
    // Wir nutzen eine RefCell für den Controller selbst, damit wir ihn in den Callbacks nutzen können
    let controller: Rc<RefCell<Option<Rc<ShellController>>>> = Rc::new(RefCell::new(None));
    let controller_cb = controller.clone();

    let on_change = move || {
        bar_ref.check_auto_hide();
    };

    let mut shell = ShellController::new(bar_popup_state, on_change);

    // Popups erstellen und im Controller registrieren
    let ctx_c = ctx.clone();
    let ctrl_l = controller_cb.clone();
    let launcher = Rc::new(LauncherPopup::new(app, ctx_c, move || {
        if let Some(c) = ctrl_l.borrow().as_ref() { c.sync(); }
    }));
    shell.add_popup(launcher.clone());

    let ctx_c = ctx.clone();
    let ctrl_q = controller_cb.clone();
    let qs = Rc::new(QuickSettingsPopup::new(app, &bar.vol_icon, ctx_c, move || {
        if let Some(c) = ctrl_q.borrow().as_ref() { c.sync(); }
    }));
    shell.add_popup(qs.clone());

    let ctx_c = ctx.clone();
    let ctrl_w = controller_cb.clone();
    let ws = Rc::new(WorkspacePopup::new(app, ctx_c, move || {
        if let Some(c) = ctrl_w.borrow().as_ref() { c.sync(); }
    }));
    shell.add_popup(ws.clone());

    let shell = Rc::new(shell);
    *controller.borrow_mut() = Some(shell.clone());

    // --- CLICK HANDLER (DRY!) ---
    setup_click_handler(&bar.launcher_island, shell.clone(), "launcher");
    setup_click_handler(&bar.status_island, shell.clone(), "qs");
    setup_click_handler(&bar.center_island, shell.clone(), "ws");

    bar.window.present();
}

fn setup_click_handler(island: &gtk4::Box, controller: Rc<ShellController>, id: &'static str) {
    let click = gtk4::GestureClick::new();
    click.connect_pressed(move |_, _, _, _| {
        controller.toggle(id);
    });
    island.add_controller(click);
}

fn setup_services() -> AppContext {
    let (network_rx, network_tx) = NetworkService::spawn();
    let (bluetooth_rx, bluetooth_tx) = BluetoothService::spawn();
    let (audio_rx, audio_tx) = AudioService::spawn();
    let audio_initial = AudioService::read_initial();
    let (backlight_rx, backlight_tx) = BacklightService::spawn();
    let backlight_initial = BacklightService::read_initial();
    let (nightlight_rx, nightlight_tx) = NightlightService::spawn();
    let nightlight_initial = NightlightService::read_initial();
    let power_rx = PowerService::spawn();
    let niri_rx = NiriService::spawn();
    let clock_rx = ClockService::spawn();

    let (launcher_tx, launcher_rx) = async_channel::unbounded();
    let launcher_store = ServiceStore::new_manual(Default::default());
    let launcher_service = LauncherService::new(launcher_store.store.clone());
    
    let launcher_service_init = launcher_service;
    glib::spawn_future_local(async move {
        launcher_service_init.add_provider(Arc::new(AppProvider::default()));
        launcher_service_init.start(launcher_rx);
    });

    AppContext {
        network: ServiceStore::new(network_rx, Default::default()),
        network_tx,
        bluetooth: ServiceStore::new(bluetooth_rx, Default::default()),
        bluetooth_tx,
        audio: ServiceStore::new(audio_rx, audio_initial),
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
    }
}
