mod app_context;
mod constants;
mod services;
mod store;
mod widgets;
mod shell;

use crate::app_context::AppContext;
use crate::services::Service;
use crate::services::audio::AudioService;
use crate::services::backlight::BacklightService;
use crate::services::bluetooth::BluetoothService;
use crate::services::clock::ClockService;
use crate::services::dnd::DndService;
use crate::services::tray::TrayService;
use crate::services::nightlight::NightlightService;
use crate::services::network::NetworkService;
use crate::services::niri::NiriService;
use crate::services::power::PowerService;
use crate::services::launcher::LauncherService;
use crate::services::ipc::IpcService;
use crate::services::notifications::NotificationService;
use crate::store::{ReadOnlyHandle, ServiceHandle};
use crate::widgets::{Bar, QuickSettingsPopup, WorkspacePopup, LauncherPopup, NotificationToastManager, osd::OsdManager};
use crate::shell::ShellController;
use gtk4::prelude::*;
use gtk4::glib;
use std::rc::Rc;
use std::cell::RefCell;

fn setup_logging() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stderr())
        .apply()
        .expect("Failed to set up logging");
}

fn main() {
    setup_logging();

    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = runtime.enter();

    log::info!("AXIS Shell starting");

    let application = libadwaita::Application::builder()
        .application_id("com.github.axis.shell")
        .build();

    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(app: &libadwaita::Application) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::PreferDark);

    let ctx = setup_services();

    // --- WIDGETS & CONTROLLER ---
    let bar = Bar::new(app, ctx.clone());
    let bar_popup_state = bar.popup_open.clone();
    let bar_ref = bar.clone();
    
    // Controller braucht RefCell für Callbacks
    let controller: Rc<RefCell<Option<Rc<ShellController>>>> = Rc::new(RefCell::new(None));
    let controller_on_change = controller.clone();

    // Toasts initialisieren
    NotificationToastManager::init(app, ctx.clone());

    // OSD initialisieren
    let _osd = OsdManager::new(app, ctx.clone());

    // Popups initialisieren (QS brauchen wir zuerst für das Archiv)
    let ctx_c = ctx.clone();
    let ctrl_q = controller.clone();
    let qs = Rc::new(QuickSettingsPopup::new(app, &bar.vol_icon, ctx_c, move || {
        if let Some(c) = ctrl_q.borrow().as_ref() { c.sync(); }
    }));

    // --- ARCHIVE (Braucht QS-Referenz) ---
    let notification_archive = crate::widgets::notification::archive::NotificationArchiveManager::new(app, ctx.clone(), &qs.container);
    let archive_cb = notification_archive.clone();

    // OnChange Callback für alle Popups
    let on_change = move || {
        bar_ref.check_auto_hide();
        if let Some(c) = controller_on_change.borrow().as_ref() {
            let is_qs = c.active_id() == Some("qs".to_string());
            archive_cb.set_visible(is_qs);
        }
    };

    let shell_ctrl = ShellController::new(bar_popup_state, on_change);
    shell_ctrl.add_popup(qs.clone());

    let ctx_c = ctx.clone();
    let ctrl_l = controller.clone();
    let launcher = Rc::new(LauncherPopup::new(app, ctx_c, move || {
        if let Some(c) = ctrl_l.borrow().as_ref() { c.sync(); }
    }));
    shell_ctrl.add_popup(launcher.clone());

    let ctx_c = ctx.clone();
    let ctrl_w = controller.clone();
    let ws = Rc::new(WorkspacePopup::new(app, ctx_c, move || {
        if let Some(c) = ctrl_w.borrow().as_ref() { c.sync(); }
    }));
    shell_ctrl.add_popup(ws.clone());

    let shell_ctrl = Rc::new(shell_ctrl);
    *controller.borrow_mut() = Some(shell_ctrl.clone());

    // --- IPC SERVICE STARTEN ---
    let ipc_rx = IpcService::spawn();
    let shell_ipc = shell_ctrl.clone();
    glib::spawn_future_local(async move {
        while let Ok(cmd) = ipc_rx.recv().await {
            use crate::services::ipc::server::ShellIpcCmd;
            match cmd {
                ShellIpcCmd::ToggleLauncher => shell_ipc.toggle("launcher"),
                ShellIpcCmd::ToggleQuickSettings => shell_ipc.toggle("qs"),
                ShellIpcCmd::ToggleWorkspaces => shell_ipc.toggle("ws"),
                ShellIpcCmd::CloseAll => shell_ipc.close_all(),
            }
        }
    });

    // --- CLICK HANDLER ---
    setup_click_handler(&bar.launcher_island, shell_ctrl.clone(), "launcher");
    setup_click_handler(&bar.status_island, shell_ctrl.clone(), "qs");
    setup_click_handler(&bar.center_island, shell_ctrl.clone(), "ws");

    log::info!("UI ready, presenting window");
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
    let (network_store, network_tx) = NetworkService::spawn();
    let (bluetooth_store, bluetooth_tx) = BluetoothService::spawn();
    let (audio_store, audio_tx) = AudioService::spawn();
    let (backlight_store, backlight_tx) = BacklightService::spawn();
    let (nightlight_store, nightlight_tx) = NightlightService::spawn();
    let (notifications_store, notifications_tx) = NotificationService::spawn();
    let (dnd_store, dnd_tx) = DndService::spawn();
    let (tray_store, tray_tx) = TrayService::spawn();
    let (power_store, _) = PowerService::spawn();
    let (niri_store, _) = NiriService::spawn();
    let (clock_store, _) = ClockService::spawn();
    let (launcher_store, launcher_tx) = LauncherService::spawn();

    AppContext {
        network: ServiceHandle { store: network_store, tx: network_tx },
        bluetooth: ServiceHandle { store: bluetooth_store, tx: bluetooth_tx },
        audio: ServiceHandle { store: audio_store, tx: audio_tx },
        backlight: ServiceHandle { store: backlight_store, tx: backlight_tx },
        nightlight: ServiceHandle { store: nightlight_store, tx: nightlight_tx },
        launcher: ServiceHandle { store: launcher_store, tx: launcher_tx },
        notifications: ServiceHandle { store: notifications_store, tx: notifications_tx },
        dnd: ServiceHandle { store: dnd_store, tx: dnd_tx },
        tray: ServiceHandle { store: tray_store, tx: tray_tx },
        power: ReadOnlyHandle { store: power_store },
        niri: ReadOnlyHandle { store: niri_store },
        clock: ReadOnlyHandle { store: clock_store },
    }
}
