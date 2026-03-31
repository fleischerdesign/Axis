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
use crate::services::continuity::ContinuityService;
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
use crate::services::settings::SettingsService;
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

    // Bidirectional Config ↔ Service sync bridge
    bridge_settings(&ctx);

    // Bluetooth pairing — reactive via store subscription (no polling)
    {
        let ctx_bt = ctx.clone();
        let last_notified: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
        ctx.bluetooth.subscribe(move |data| {
            match &data.pairing_request {
                Some(req) if *last_notified.borrow() != req.device_name => {
                    *last_notified.borrow_mut() = req.device_name.clone();
                    crate::widgets::quick_settings::bluetooth_pair_dialog::send_pairing_notification(
                        req,
                        ctx_bt.bluetooth.tx.clone(),
                        &ctx_bt.notifications.tx,
                    );
                }
                None => {
                    last_notified.borrow_mut().clear();
                }
                _ => {}
            }
        });
    }

    // Continuity pairing — reactive via store subscription
    {
        let ctx_c = ctx.clone();
        let last_notified: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
        ctx.continuity.subscribe(move |data| {
            if let Some(pending) = &data.pending_pin {
                // Only show notification for INCOMING requests
                if !pending.is_incoming {
                    return;
                }

                let peer_id = &pending.peer_id;
                if last_notified.borrow().as_deref() != Some(peer_id) {
                    *last_notified.borrow_mut() = Some(peer_id.clone());
                    
                    let tx_c = ctx_c.continuity.tx.clone();
                    
                    let mut on_action: std::collections::HashMap<String, std::sync::Arc<dyn Fn() + Send + Sync>> = std::collections::HashMap::new();
                    
                    on_action.insert(
                        "accept".to_string(),
                        std::sync::Arc::new({
                            let tx = tx_c.clone();
                            move || { let _ = tx.try_send(crate::services::continuity::ContinuityCmd::ConfirmPin); }
                        })
                    );
                    on_action.insert(
                        "reject".to_string(),
                        std::sync::Arc::new({
                            let tx = tx_c.clone();
                            move || { let _ = tx.try_send(crate::services::continuity::ContinuityCmd::RejectPin); }
                        })
                    );

                    let body = if pending.is_incoming {
                        format!("Kopplungsanfrage von {}\nPIN: {}", pending.peer_name, pending.pin)
                    } else {
                        format!("Bitte bestätigen Sie die PIN {} auf dem Gerät {}", pending.pin, pending.peer_name)
                    };

                    let notification = crate::services::notifications::Notification {
                        id: 4294967294, // distinct from bluetooth
                        app_name: "Continuity".to_string(),
                        app_icon: "computer-symbolic".to_string(),
                        summary: "Gerätekopplung".to_string(),
                        body,
                        urgency: 2,
                        timestamp: chrono::Local::now().timestamp(),
                        actions: vec![
                            crate::services::notifications::NotificationAction { key: "accept".to_string(), label: "Bestätigen".to_string() },
                            crate::services::notifications::NotificationAction { key: "reject".to_string(), label: "Ablehnen".to_string() },
                        ],
                        on_action: Some(on_action),
                        internal_id: 2,
                    };
                    
                    let _ = ctx_c.notifications.tx.try_send(crate::services::notifications::server::NotificationCmd::Show(notification));
                }
            } else {
                *last_notified.borrow_mut() = None;
                // Optionally close the notification if pairing ends, but for now we just clear the state.
            }
        });
    }

    // --- WIDGETS & CONTROLLER ---
    let bar = Bar::new(app, ctx.clone());
    let bar_popup_state = bar.popup_open.clone();

    // Continuity Capture Controller
    let _continuity_ctrl = crate::widgets::continuity_capture::ContinuityCaptureController::new(app, ctx.clone());

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
    let archive_mgr = notification_archive.clone();
    qs.base.is_open.subscribe(move |&is_open| {
        archive_mgr.set_popup_open(is_open);
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

    // --- SETTINGS D-BUS SERVER ---
    // Must run on the SAME tokio runtime as IPC (main thread), otherwise the
    // bus name "org.axis.Shell" can't be shared between connections.
    {
        use std::sync::{Arc, Mutex};

        let settings_config = Arc::new(Mutex::new(ctx.settings.store.get().config));
        let (notify_tx, notify_rx) = async_channel::unbounded::<()>();

        let cache = settings_config.clone();
        ctx.settings.store.subscribe(move |data| {
            *cache.lock().unwrap() = data.config.clone();
            let _ = notify_tx.send(());
        });

        let settings_tx = ctx.settings.tx.clone();
        tokio::spawn(async move {
            use zbus::connection::Builder;
            let server = crate::services::settings::dbus::SettingsDbusServer::new(
                settings_tx,
                settings_config.clone(),
            );
            let conn_res = async {
                let builder = Builder::session()?;
                let builder = builder.name("org.axis.Shell")?;
                let builder = builder.serve_at("/org/axis/Shell/Settings", server)?;
                builder.build().await
            }
            .await;

            match conn_res {
                Ok(conn) => {
                    log::info!("[settings] D-Bus Interface 'org.axis.Shell.Settings' registered");
                    while let Ok(()) = notify_rx.recv().await {
                        let json = serde_json::to_string(&*settings_config.lock().unwrap())
                            .unwrap_or_default();
                        let iface = conn
                            .object_server()
                            .interface::<_, crate::services::settings::dbus::SettingsDbusServer>(
                                "/org/axis/Shell/Settings",
                            )
                            .await;
                        if let Ok(iface) = iface {
                            let _ = crate::services::settings::dbus::SettingsDbusServer::settings_changed(
                                iface.signal_emitter(),
                                "all",
                                &json,
                            )
                            .await;
                        }
                    }
                }
                Err(e) => log::error!("[settings] Failed to register D-Bus interface: {:?}", e),
            }
        });
    }

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
    use crate::app_context::{spawn_readonly, spawn_service};
    use crate::services::calendar::CalendarRegistry;

    AppContext {
        airplane:     spawn_service::<AirplaneService>(),
        network:      spawn_service::<NetworkService>(),
        bluetooth:    spawn_service::<BluetoothService>(),
        audio:        spawn_service::<AudioService>(),
        backlight:    spawn_service::<BacklightService>(),
        nightlight:   spawn_service::<NightlightService>(),
        launcher:     spawn_service::<LauncherService>(),
        notifications: spawn_service::<NotificationService>(),
        dnd:          spawn_service::<DndService>(),
        tray:         spawn_service::<TrayService>(),
        kdeconnect:   spawn_service::<KdeConnectService>(),
        power:        spawn_readonly::<PowerService>(),
        niri:         spawn_readonly::<NiriService>(),
        continuity:    spawn_service::<ContinuityService>(),
        settings:      spawn_service::<SettingsService>(),
        clock:        spawn_readonly::<ClockService>(),
        task_registry: Arc::new(Mutex::new(TaskRegistry::new())),
        calendar_registry: Arc::new(Mutex::new(CalendarRegistry::new())),
    }
}

fn bridge_settings(ctx: &AppContext) {
    use crate::services::settings::sync;
    use crate::services::dnd::DndService;
    use crate::services::airplane::AirplaneService;
    use crate::services::bluetooth::BluetoothService;
    use crate::services::continuity::ContinuityService;

    // Generic bidirectional sync for all ServiceConfig services
    sync::wire_service_config_full::<DndService>(
        &ctx.settings.store, &ctx.dnd.store, &ctx.dnd.tx, &ctx.settings.tx,
        |c| c.services.dnd_enabled, |c, v| c.services.dnd_enabled = v,
    );

    sync::wire_service_config_full::<AirplaneService>(
        &ctx.settings.store, &ctx.airplane.store, &ctx.airplane.tx, &ctx.settings.tx,
        |c| c.services.airplane_enabled, |c, v| c.services.airplane_enabled = v,
    );

    sync::wire_service_config_full::<BluetoothService>(
        &ctx.settings.store, &ctx.bluetooth.store, &ctx.bluetooth.tx, &ctx.settings.tx,
        |c| c.services.bluetooth_enabled, |c, v| c.services.bluetooth_enabled = v,
    );

    sync::wire_service_config_full::<ContinuityService>(
        &ctx.settings.store, &ctx.continuity.store, &ctx.continuity.tx, &ctx.settings.tx,
        |c| c.continuity.enabled, |c, v| c.continuity.enabled = v,
    );

    // Nightlight: custom sync (config has multiple fields, not just enabled)
    sync::wire_nightlight_config_sync(
        &ctx.settings.store, &ctx.nightlight.tx,
    );
}
