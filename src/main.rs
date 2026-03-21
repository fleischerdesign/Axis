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
use crate::widgets::{Bar, QuickSettingsPopup, WorkspacePopup, LauncherPopup, NotificationToastManager, osd::OsdManager};
use crate::shell::ShellController;
use gtk4::prelude::*;
use gtk4::glib;
use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = runtime.enter();

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
    let (network, network_tx) = NetworkService::spawn();
    let (bluetooth, bluetooth_tx) = BluetoothService::spawn();
    let (audio, audio_tx) = AudioService::spawn();
    let (backlight, backlight_tx) = BacklightService::spawn();
    let (nightlight, nightlight_tx) = NightlightService::spawn();
    let (notifications, notifications_tx) = NotificationService::spawn();
    let (power, _) = PowerService::spawn();
    let (niri, _) = NiriService::spawn();
    let (clock, _) = ClockService::spawn();
    let (launcher, launcher_tx) = LauncherService::spawn();
    let (dnd, dnd_tx) = DndService::spawn();
    let (tray, tray_tx) = TrayService::spawn();

    AppContext {
        network, network_tx,
        bluetooth, bluetooth_tx,
        audio, audio_tx,
        backlight, backlight_tx,
        nightlight, nightlight_tx,
        launcher, launcher_tx,
        notifications, notifications_tx,
        power, niri, clock,
        dnd, dnd_tx,
        tray, tray_tx,
    }
}
