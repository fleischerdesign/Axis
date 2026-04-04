mod app_context;
mod services;
mod widgets;
mod shell;

use axis_core::constants;
use crate::app_context::AppContext;
use axis_core::services::audio::AudioService;
use axis_core::services::airplane::AirplaneService;
use axis_core::services::backlight::BacklightService;
use axis_core::services::tasks::TaskRegistry;
use axis_core::services::bluetooth::BluetoothService;
use axis_core::services::clock::ClockService;
use axis_core::services::continuity::ContinuityService;
use axis_core::services::dnd::DndService;
use axis_core::services::kdeconnect::KdeConnectService;
use axis_core::services::tray::TrayService;
use axis_core::services::nightlight::NightlightService;
use axis_core::services::network::NetworkService;
use crate::services::niri::NiriService;
use axis_core::services::power::PowerService;
use crate::services::launcher::LauncherService;
use crate::services::notifications::NotificationService;
use axis_core::services::settings::SettingsService;
use axis_core::services::settings::config::AccentColor;
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

    let start_locked = std::env::args().any(|a| a == "--locked");
    let args_vec: Vec<String> = std::env::args().collect();
    let wallpaper_path = args_vec
        .windows(2)
        .find(|w| w[0] == "--wallpaper")
        .map(|w| w[1].clone());

    let application = libadwaita::Application::builder()
        .application_id("com.github.axis.shell")
        .build();

    application.connect_activate(move |app| {
        AppShell::build(app, start_locked, wallpaper_path.clone());
    });

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

// ── AppShell ──────────────────────────────────────────────────────────────
//
// Encapsulates the entire shell initialization as a sequence of explicit
// phases. Each phase is a focused method with clear inputs/outputs.

struct AppShell;

impl AppShell {
    fn build(
        app: &libadwaita::Application,
        start_locked: bool,
        wallpaper_path: Option<String>,
    ) {
        // Phase 1: Theme infrastructure
        let theme_provider = Self::setup_theme();

        // Phase 2: Services + config bridge
        let ctx = Self::setup_services();
        Self::bridge_settings(&ctx);

        // Phase 3: Initial appearance
        Self::apply_initial_appearance(&ctx, &theme_provider);
        Self::force_dark_mode();

        // Phase 4: Notification subscriptions
        Self::subscribe_bluetooth_notifications(&ctx);
        Self::subscribe_continuity_notifications(&ctx);

        // Phase 5: Core widgets
        let (bar, wallpaper_svc, lock_screen) =
            Self::setup_widgets(app, &ctx, &wallpaper_path);

        // Phase 6: Reactive appearance subscriber
        Self::subscribe_appearance_changes(
            &ctx,
            &theme_provider,
            &wallpaper_svc,
            &lock_screen,
            &wallpaper_path,
        );

        // Phase 7: Shell integration
        NotificationToastManager::init(app, ctx.clone());
        let _osd = OsdManager::new(app, ctx.clone());
        let shell_ctrl = Self::setup_shell(&bar, &ctx);
        Self::register_popups(app, &ctx, &shell_ctrl, &lock_screen, &bar);
        Self::spawn_dbus(&ctx, shell_ctrl.clone(), lock_screen.clone());
        Self::wire_click_handlers(&bar, shell_ctrl.clone());

        // Phase 8: Boot state
        bar.window.present();
        Self::maybe_lock(&lock_screen, start_locked);
    }

    // ── Phase 1: Theme ────────────────────────────────────────────────

    fn setup_theme() -> gtk4::CssProvider {
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));

        let theme_provider = gtk4::CssProvider::new();
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            gtk4::style_context_add_provider_for_display(
                &display,
                &theme_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }
        theme_provider
    }

    // ── Phase 2: Services ─────────────────────────────────────────────

    fn setup_services() -> AppContext {
        use crate::app_context::{spawn_readonly, spawn_service};
        use axis_core::services::calendar::CalendarRegistry;

        AppContext {
            airplane:       spawn_service::<AirplaneService>(),
            network:        spawn_service::<NetworkService>(),
            bluetooth:      spawn_service::<BluetoothService>(),
            audio:          spawn_service::<AudioService>(),
            backlight:      spawn_service::<BacklightService>(),
            nightlight:     spawn_service::<NightlightService>(),
            launcher:       spawn_service::<LauncherService>(),
            notifications:  spawn_service::<NotificationService>(),
            dnd:            spawn_service::<DndService>(),
            tray:           spawn_service::<TrayService>(),
            kdeconnect:     spawn_service::<KdeConnectService>(),
            power:          spawn_readonly::<PowerService>(),
            niri:           spawn_readonly::<NiriService>(),
            continuity:     spawn_service::<ContinuityService>(),
            settings:       spawn_service::<SettingsService>(),
            clock:          spawn_readonly::<ClockService>(),
            task_registry:  Arc::new(Mutex::new(TaskRegistry::new())),
            calendar_registry: Arc::new(Mutex::new(CalendarRegistry::new())),
        }
    }

    fn bridge_settings(ctx: &AppContext) {
        use axis_core::services::settings::sync;
        use axis_core::services::dnd::DndService;
        use axis_core::services::airplane::AirplaneService;
        use axis_core::services::bluetooth::BluetoothService;

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
        sync::wire_continuity_sync(
            &ctx.settings.store, &ctx.continuity.store, &ctx.continuity.tx, &ctx.settings.tx,
        );
        sync::wire_nightlight_config_sync(
            &ctx.settings.store, &ctx.nightlight.tx,
        );
    }

    // ── Phase 3: Initial Appearance ───────────────────────────────────

    fn apply_initial_appearance(ctx: &AppContext, theme_provider: &gtk4::CssProvider) {
        let config = ctx.settings.store.get();
        let appearance = &config.config.appearance;

        let accent = match &appearance.accent_color {
            AccentColor::Auto => appearance.wallpaper.as_ref()
                .and_then(|p| crate::widgets::wallpaper::extract_accent_color(p))
                .unwrap_or_else(|| AccentColor::Blue.hex_value().to_string()),
            c => c.hex_value().to_string(),
        };
        update_theme_css(theme_provider, &accent, &appearance.font);
        write_accent_css(&accent);
    }

    fn force_dark_mode() {
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::ForceDark);
        libadwaita::StyleManager::default().connect_notify_local(Some("dark"), |mgr, _| {
            log::info!("[theme] changed: dark={}", mgr.is_dark());
        });
    }

    // ── Phase 4: Notification Subscriptions ───────────────────────────

    fn subscribe_bluetooth_notifications(ctx: &AppContext) {
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

    fn subscribe_continuity_notifications(ctx: &AppContext) {
        let ctx_c = ctx.clone();
        let last_notified: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
        let last_connected: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
        ctx.continuity.subscribe(move |data| {
            // Pairing
            if let Some(pending) = &data.pending_pin {
                if !pending.is_incoming {
                    return;
                }
                let peer_id = &pending.peer_id;
                if last_notified.borrow().as_deref() != Some(peer_id) {
                    *last_notified.borrow_mut() = Some(peer_id.clone());
                    crate::widgets::quick_settings::continuity_pair_dialog::send_pairing_notification(
                        &pending.peer_name,
                        &pending.pin,
                        pending.is_incoming,
                        ctx_c.continuity.tx.clone(),
                        &ctx_c.notifications.tx,
                    );
                }
            } else {
                *last_notified.borrow_mut() = None;
            }

            // Auto-connect for trusted peers
            if let Some(conn) = &data.active_connection {
                if data.peer_configs.get(&conn.peer_id).is_some_and(|c| c.trusted) {
                    if last_connected.borrow().as_deref() != Some(&conn.peer_id) {
                        *last_connected.borrow_mut() = Some(conn.peer_id.clone());
                        crate::widgets::quick_settings::continuity_pair_dialog::send_connected_notification(
                            &conn.peer_name,
                            &ctx_c.notifications.tx,
                        );
                    }
                } else {
                    last_connected.borrow_mut().take();
                }
            } else {
                last_connected.borrow_mut().take();
            }
        });
    }

    // ── Phase 5: Core Widgets ─────────────────────────────────────────

    fn setup_widgets(
        app: &libadwaita::Application,
        ctx: &AppContext,
        wallpaper_path: &Option<String>,
    ) -> (Bar, Rc<crate::widgets::wallpaper::WallpaperService>, Rc<LockScreen>) {
        let bar = Bar::new(app, ctx.clone());

        let _continuity_ctrl =
            crate::widgets::continuity_capture::ContinuityCaptureController::new(app, ctx.clone());

        let wallpaper_svc = Rc::new(crate::widgets::wallpaper::WallpaperService::new(app));
        let wallpaper_cfg = ctx.settings.store.get().config.appearance.wallpaper.clone();
        let wallpaper_resolved = wallpaper_cfg.or(wallpaper_path.clone());
        let lockscreen_texture = wallpaper_resolved
            .as_ref()
            .and_then(|p| wallpaper_svc.show(p));

        let lock_screen = Rc::new(LockScreen::new(lockscreen_texture, ctx.power.clone()));
        setup_lock_triggers(&lock_screen);

        (bar, wallpaper_svc, lock_screen)
    }

    // ── Phase 6: Reactive Appearance ──────────────────────────────────

    fn subscribe_appearance_changes(
        ctx: &AppContext,
        theme_provider: &gtk4::CssProvider,
        wallpaper_svc: &Rc<crate::widgets::wallpaper::WallpaperService>,
        lock_screen: &Rc<LockScreen>,
        wallpaper_path: &Option<String>,
    ) {
        let theme_provider_c = theme_provider.clone();
        let wallpaper_path_c = wallpaper_path.clone();
        let wallpaper_svc_c = wallpaper_svc.clone();
        let lock_screen_c = lock_screen.clone();
        let last_wallpaper: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let last_accent_for_css: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

        ctx.settings.store.subscribe(move |data| {
            let app_cfg = &data.config.appearance;

            // Wallpaper
            let resolved_wp = app_cfg.wallpaper.as_ref()
                .or(wallpaper_path_c.as_ref())
                .cloned();
            if *last_wallpaper.borrow() != resolved_wp {
                *last_wallpaper.borrow_mut() = resolved_wp.clone();
                let new_texture = wallpaper_svc_c.set_wallpaper(resolved_wp.as_deref());
                lock_screen_c.set_wallpaper(new_texture);
            }

            // Accent + font
            let accent = match &app_cfg.accent_color {
                AccentColor::Auto => {
                    let path = app_cfg.wallpaper.as_ref()
                        .or(wallpaper_path_c.as_ref());
                    path.and_then(|p| crate::widgets::wallpaper::extract_accent_color(p))
                        .unwrap_or_else(|| AccentColor::Blue.hex_value().to_string())
                }
                c => c.hex_value().to_string(),
            };
            update_theme_css(&theme_provider_c, &accent, &app_cfg.font);

            if last_accent_for_css.borrow().as_deref() != Some(accent.as_str()) {
                *last_accent_for_css.borrow_mut() = Some(accent.clone());
                write_accent_css(&accent);
            }
        });
    }

    // ── Phase 7: Shell Integration ────────────────────────────────────

    fn setup_shell(bar: &Bar, ctx: &AppContext) -> Rc<ShellController> {
        let bar_ref = bar.clone();
        let on_change = move || {
            bar_ref.check_auto_hide();
        };
        Rc::new(ShellController::new(bar.popup_open.clone(), on_change))
    }

    fn register_popups(
        app: &libadwaita::Application,
        ctx: &AppContext,
        shell_ctrl: &Rc<ShellController>,
        lock_screen: &Rc<LockScreen>,
        bar: &Bar,
    ) {
        let ls_lock = lock_screen.clone();
        let qs = Rc::new(QuickSettingsPopup::new(
            app,
            bar.volume_icon(),
            ctx.clone(),
            Rc::new(move || ls_lock.lock_session()) as Rc<dyn Fn()>,
        ));
        shell_ctrl.register(&qs);

        // Archive overlay
        let notification_archive =
            crate::widgets::notification::archive::NotificationArchiveManager::new(ctx.clone());
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
    }

    fn spawn_dbus(
        ctx: &AppContext,
        shell_ctrl: Rc<ShellController>,
        lock_screen: Rc<LockScreen>,
    ) {
        crate::shell::dbus::spawn_dbus_host(ctx, shell_ctrl, lock_screen);
    }

    fn wire_click_handlers(bar: &Bar, shell_ctrl: Rc<ShellController>) {
        setup_click_handler(bar.launcher_island(), shell_ctrl.clone(), "launcher");
        setup_click_handler(bar.status_island(), shell_ctrl.clone(), "qs");
        setup_click_handler(bar.workspace_island(), shell_ctrl.clone(), "ws");
        setup_click_handler(bar.clock_island(), shell_ctrl, "calendar");
    }

    // ── Phase 8: Boot State ───────────────────────────────────────────

    fn maybe_lock(lock_screen: &Rc<LockScreen>, start_locked: bool) {
        if start_locked {
            log::info!("[main] --locked flag detected, locking session");
            let ls = lock_screen.clone();
            glib::idle_add_local_once(move || {
                ls.lock_session();
            });
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn setup_click_handler(island: &gtk4::Box, controller: Rc<ShellController>, id: &'static str) {
    let click = gtk4::GestureClick::new();
    click.connect_pressed(move |_, _, _, _| {
        controller.toggle(id);
    });
    island.add_controller(click);
}

fn setup_lock_triggers(lock_screen: &Rc<LockScreen>) {
    let (lock_tx, lock_rx) = async_channel::bounded::<()>(1);

    let ls = lock_screen.clone();
    glib::spawn_future_local(async move {
        while lock_rx.recv().await.is_ok() {
            if !ls.is_locked() {
                log::info!("[lock] Signal received, locking session");
                ls.lock_session();
            }
        }
    });

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

fn update_theme_css(provider: &gtk4::CssProvider, accent_hex: &str, font: &Option<String>) {
    let hover = crate::widgets::wallpaper::lighten_hex(accent_hex, 0.15);
    let font_rule = font.as_ref()
        .map(|f| format!("window {{ --font-family: \"{f}\"; }}"))
        .unwrap_or_default();
    let css = format!(
        "@define-color accent_bg_color {accent_hex};\n\
         @define-color accent_fg_color #ffffff;\n\
         @define-color accent_hover_color {hover};\n\
         {font_rule}"
    );
    provider.load_from_string(&css);
}

fn write_accent_css(accent_hex: &str) {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            format!("{home}/.config")
        });

    let css = format!(
        "@define-color accent_bg_color {accent_hex};\n\
         @define-color accent_color {accent_hex};\n\
         @define-color accent_fg_color #ffffff;\n"
    );

    for gtk_ver in &["gtk-4.0", "gtk-3.0"] {
        let dir = format!("{config_dir}/{gtk_ver}");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::warn!("[theme] Failed to create {dir}: {e}");
            continue;
        }
        if let Err(e) = std::fs::write(format!("{dir}/gtk.css"), &css) {
            log::warn!("[theme] Failed to write {dir}/gtk.css: {e}");
        }
    }
}
