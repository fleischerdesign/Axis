mod app_context;
mod constants;
mod services;
mod store;
mod widgets;
mod shell;

use crate::app_context::AppContext;
use crate::services::Service;
use crate::services::audio::AudioService;
use crate::services::airplane::AirplaneService;
use crate::services::backlight::BacklightService;
use crate::services::tasks::TaskRegistry;
use crate::services::bluetooth::BluetoothService;
use crate::services::clock::ClockService;
use crate::services::dnd::DndService;
use crate::services::kdeconnect::KdeConnectService;
use crate::services::tray::TrayService;
use crate::services::nightlight::NightlightService;
use crate::services::network::NetworkService;
use crate::services::niri::NiriService;
use crate::services::power::PowerService;
use crate::services::launcher::LauncherService;
use crate::services::ipc::IpcService;
use crate::services::notifications::NotificationService;
use crate::store::{ReadOnlyHandle, ServiceHandle};
use crate::widgets::{Bar, QuickSettingsPopup, WorkspacePopup, LauncherPopup, CalendarPopup, NotificationToastManager, osd::OsdManager};
use crate::widgets::lock_screen::LockScreen;
use crate::shell::ShellController;
use gtk4::prelude::*;
use gtk4::glib;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

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

    // Check for AXIS-specific flags before GTK processes args
    let start_locked = std::env::args().any(|a| a == "--locked");
    let args_vec: Vec<String> = std::env::args().collect();
    let wallpaper_path = args_vec
        .windows(2)
        .find(|w| w[0] == "--wallpaper")
        .map(|w| w[1].clone());

    let application = libadwaita::Application::builder()
        .application_id("com.github.axis.shell")
        .build();

    application.connect_activate(move |app| build_ui(app, start_locked, wallpaper_path.clone()));

    // Filter out AXIS-specific flags before passing to GTK
    let skip_next = std::cell::Cell::new(false);
    let args: Vec<String> = std::env::args()
        .filter(|a| {
            if skip_next.take() {
                return false;
            }
            if a == "--locked" {
                return false;
            }
            if a == "--wallpaper" {
                skip_next.set(true);
                return false;
            }
            true
        })
        .collect();
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    application.run_with_args(&args_ref);
}

fn build_ui(app: &libadwaita::Application, start_locked: bool, wallpaper_path: Option<String>) {
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

    // Bluetooth pairing via notification system
    let ctx_bt = ctx.clone();
    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
        let state = crate::services::bluetooth::get_pairing_ui_state();

        // Close stale notification
        if state.should_close_notif && state.notif_id > 0 {
            let _ = ctx_bt.notifications.tx.try_send(
                crate::services::notifications::server::NotificationCmd::Close(state.notif_id)
            );
            crate::services::bluetooth::set_pairing_notification_id(0);
        }

        // Show new notification
        if let Some(ref req) = state.request {
            if state.notif_id == 0 {
                crate::widgets::quick_settings::bluetooth_pair_dialog::send_pairing_notification(
                    req,
                    ctx_bt.bluetooth.tx.clone(),
                    &ctx_bt.notification_raw_tx,
                );
            }
        }

        gtk4::glib::ControlFlow::Continue
    });

    // --- WIDGETS & CONTROLLER ---
    let bar = Bar::new(app, ctx.clone());
    let bar_popup_state = bar.popup_open.clone();

    // Wallpaper
    let lockscreen_texture = wallpaper_path
        .as_ref()
        .and_then(|p| crate::widgets::wallpaper::WallpaperService::show(app, p));

    // Lock Screen
    let lock_screen = Rc::new(LockScreen::new(lockscreen_texture, ctx.power.clone()));
    setup_lock_triggers(&lock_screen);

    // Controller für IPC
    let controller: Rc<RefCell<Option<Rc<ShellController>>>> = Rc::new(RefCell::new(None));

    // Toasts initialisieren
    NotificationToastManager::init(app, ctx.clone());

    // OSD initialisieren
    let _osd = OsdManager::new(app, ctx.clone());

    // OnChange Callback
    let bar_ref = bar.clone();
    let on_change = move || {
        bar_ref.check_auto_hide();
    };

    let shell_ctrl = Rc::new(ShellController::new(bar_popup_state, on_change));
    *controller.borrow_mut() = Some(shell_ctrl.clone());

    // Popups registrieren
    let ls_lock = lock_screen.clone();
    let qs = Rc::new(QuickSettingsPopup::new(app, bar.volume_icon(), ctx.clone(), Rc::new(move || ls_lock.lock_session()) as Rc<dyn Fn()>));
    shell_ctrl.register(&qs);

    // --- ARCHIVE (über dem QS Popup) ---
    let notification_archive = crate::widgets::notification::archive::NotificationArchiveManager::new(ctx.clone());
    qs.archive_box.append(&notification_archive.container);
    let archive_cb = notification_archive.clone();
    let shell_ctrl_for_archive = shell_ctrl.clone();
    qs.base.is_open.subscribe(move |_is_open| {
        let is_qs = shell_ctrl_for_archive.active_id() == Some("qs".to_string());
        archive_cb.set_visible(is_qs);
    });

    let launcher = Rc::new(LauncherPopup::new(app, ctx.clone()));
    shell_ctrl.register(&launcher);

    let ws = Rc::new(WorkspacePopup::new(app, ctx.clone()));
    shell_ctrl.register(&ws);

    let cal = Rc::new(CalendarPopup::new(app, ctx.clone()));
    shell_ctrl.register(&cal);

    // --- IPC SERVICE STARTEN ---
    let ipc_rx = IpcService::spawn();
    let shell_ipc = shell_ctrl.clone();
    let ipc_lock = lock_screen.clone();
    glib::spawn_future_local(async move {
        while let Ok(cmd) = ipc_rx.recv().await {
            use crate::services::ipc::server::ShellIpcCmd;
            match cmd {
                ShellIpcCmd::ToggleLauncher => shell_ipc.toggle("launcher"),
                ShellIpcCmd::ToggleQuickSettings => shell_ipc.toggle("qs"),
                ShellIpcCmd::ToggleWorkspaces => shell_ipc.toggle("ws"),
                ShellIpcCmd::CloseAll => shell_ipc.close_all(),
                ShellIpcCmd::Lock => ipc_lock.lock_session(),
            }
        }
    });

    // --- CLICK HANDLER ---
    setup_click_handler(bar.launcher_island(), shell_ctrl.clone(), "launcher");
    setup_click_handler(bar.status_island(), shell_ctrl.clone(), "qs");
    setup_click_handler(bar.workspace_island(), shell_ctrl.clone(), "ws");
    setup_click_handler(bar.clock_island(), shell_ctrl.clone(), "calendar");

    log::info!("UI ready, presenting window");
    bar.window.present();

    // Lock on boot if --locked flag was passed
    if start_locked {
        log::info!("[main] --locked flag detected, locking session");
        let ls = lock_screen.clone();
        glib::idle_add_local_once(move || {
            ls.lock_session();
        });
    }
}

fn setup_click_handler(island: &gtk4::Box, controller: Rc<ShellController>, id: &'static str) {
    let click = gtk4::GestureClick::new();
    click.connect_pressed(move |_, _, _, _| {
        controller.toggle(id);
    });
    island.add_controller(click);
}

/// Listens for logind signals that should trigger the lock screen:
/// - PrepareForSleep(true): lock before suspend
/// - Lock signal on the session: lock on idle / loginctl lock-session
fn setup_lock_triggers(lock_screen: &Rc<LockScreen>) {
    let (lock_tx, lock_rx) = async_channel::bounded::<()>(1);

    // Main-thread handler: receives lock triggers and executes lock
    let ls = lock_screen.clone();
    glib::spawn_future_local(async move {
        while lock_rx.recv().await.is_ok() {
            if !ls.is_locked() {
                log::info!("[lock] Signal received, locking session");
                ls.lock_session();
            }
        }
    });

    // D-Bus listener runs on tokio runtime
    let trigger = move || {
        let _ = lock_tx.try_send(());
    };

    tokio::spawn(async move {
        use futures_util::StreamExt;

        let Ok(conn) = zbus::Connection::system().await else {
            log::warn!("[lock] Failed to connect to system D-Bus for logind signals");
            return;
        };

        let manager_proxy = match zbus::Proxy::new(
            &conn,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                log::warn!("[lock] Failed to create logind manager proxy: {e}");
                return;
            }
        };

        let Ok(mut sleep_stream) = manager_proxy.receive_signal("PrepareForSleep").await else {
            log::warn!("[lock] Failed to subscribe to PrepareForSleep signal");
            return;
        };

        let session_path: Option<zbus::zvariant::OwnedObjectPath> =
            manager_proxy.get_property("Session").await.ok();

        let mut lock_stream_opt = if let Some(ref path) = session_path {
            match zbus::Proxy::new(
                &conn,
                "org.freedesktop.login1",
                path.as_str(),
                "org.freedesktop.login1.Session",
            )
            .await
            {
                Ok(session_proxy) => session_proxy.receive_signal("Lock").await.ok(),
                Err(e) => {
                    log::warn!("[lock] Failed to create session proxy: {e}");
                    None
                }
            }
        } else {
            log::warn!("[lock] Could not determine session path");
            None
        };

        log::info!("[lock] Listening for logind signals");

        loop {
            if let Some(ref mut lock_stream) = lock_stream_opt {
                tokio::select! {
                    Some(msg) = sleep_stream.next() => {
                        if let Ok(body) = msg.body().deserialize::<bool>() {
                            if body {
                                trigger();
                            }
                        }
                    }
                    Some(_msg) = lock_stream.next() => {
                        trigger();
                    }
                }
            } else {
                if let Some(msg) = sleep_stream.next().await {
                    if let Ok(body) = msg.body().deserialize::<bool>() {
                        if body {
                            trigger();
                        }
                    }
                }
            }
        }
    });
}

fn setup_services() -> AppContext {
    let (airplane_store, airplane_tx) = AirplaneService::spawn();
    let (network_store, network_tx) = NetworkService::spawn();
    let (bluetooth_store, bluetooth_tx) = BluetoothService::spawn();
    let (audio_store, audio_tx) = AudioService::spawn();
    let (backlight_store, backlight_tx) = BacklightService::spawn();
    let (nightlight_store, nightlight_tx) = NightlightService::spawn();
    let (notifications_store, notifications_tx, notification_raw_tx) = NotificationService::spawn_with_raw_tx();
    let (dnd_store, dnd_tx) = DndService::spawn();
    let (tray_store, tray_tx) = TrayService::spawn();
    let (kdeconnect_store, kdeconnect_tx) = KdeConnectService::spawn();
    let (power_store, _) = PowerService::spawn();
    let (niri_store, _) = NiriService::spawn();
    let (clock_store, _) = ClockService::spawn();
    let (launcher_store, launcher_tx) = LauncherService::spawn();
    let task_registry = Arc::new(Mutex::new(TaskRegistry::new()));

    AppContext {
        airplane: ServiceHandle { store: airplane_store, tx: airplane_tx },
        network: ServiceHandle { store: network_store, tx: network_tx },
        bluetooth: ServiceHandle { store: bluetooth_store, tx: bluetooth_tx },
        audio: ServiceHandle { store: audio_store, tx: audio_tx },
        backlight: ServiceHandle { store: backlight_store, tx: backlight_tx },
        nightlight: ServiceHandle { store: nightlight_store, tx: nightlight_tx },
        launcher: ServiceHandle { store: launcher_store, tx: launcher_tx },
        notifications: ServiceHandle { store: notifications_store, tx: notifications_tx },
        notification_raw_tx,
        dnd: ServiceHandle { store: dnd_store, tx: dnd_tx },
        tray: ServiceHandle { store: tray_store, tx: tray_tx },
        kdeconnect: ServiceHandle { store: kdeconnect_store, tx: kdeconnect_tx },
        power: ReadOnlyHandle { store: power_store },
        niri: ReadOnlyHandle { store: niri_store },
        clock: ReadOnlyHandle { store: clock_store },
        task_registry,
    }
}
