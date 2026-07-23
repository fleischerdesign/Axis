use super::presenters::Presenters;
use super::providers::Providers;
use super::use_cases::UseCases;
use crate::services::dbus_host;
use crate::services::theme_service::ThemeService;
use crate::services::wallpaper_service::WallpaperService;
use crate::widgets::agenda::AgendaPopup;
use crate::widgets::bar_window::BarWindow;
use crate::widgets::components::power_actions::PowerActionStack;
use crate::widgets::launcher_popup::LauncherPopup;
use crate::widgets::lock_screen::LockScreenFactory;
use crate::widgets::mpris_popup::MprisPopup;
use crate::widgets::notification_archive::NotificationArchive;
use crate::widgets::notification_toast::NotificationToastManager;
use crate::widgets::osd::OsdManager;
use crate::widgets::quick_settings::QuickSettingsPopup;
use axis_application::use_cases::layout::set_border::SetBorderColorUseCase;
use axis_application::use_cases::popups::TogglePopupUseCase;
use axis_domain::models::appearance::AccentColor;
use axis_domain::models::dnd::DndStatus;
use axis_domain::models::popups::PopupType;
use axis_infrastructure::adapters::lock::LockGtkHandle;
use axis_infrastructure::adapters::niri_layout::NiriLayoutProvider;
use axis_presentation::FnView;
use axis_presentation::Presenter;
use gtk4::glib;
use libadwaita::prelude::*;
use std::cell::OnceCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

pub struct WiringArgs<'a> {
    pub app: &'a libadwaita::Application,
    pub p: &'a Providers,
    pub uc: &'a UseCases,
    pub pres: &'a Presenters,
    pub rt: &'a tokio::runtime::Runtime,
    pub theme_provider: Rc<OnceCell<Rc<gtk4::CssProvider>>>,
    pub lock_gtk_handle: LockGtkHandle,
    pub start_locked: bool,
}

pub fn wire(args: WiringArgs) {
    let WiringArgs {
        app,
        p,
        uc,
        pres,
        rt,
        theme_provider,
        lock_gtk_handle,
        start_locked,
    } = args;

    let theme_css = theme_provider.get().cloned().unwrap_or_else(|| {
        log::error!("theme provider not initialized, falling back to empty CSS");
        Rc::new(gtk4::CssProvider::new())
    });
    let theme_svc = Rc::new(ThemeService::new(theme_css));
    let wallpaper_svc = Rc::new(WallpaperService::new(app));
    let config_dir = dirs::config_dir()
        .unwrap_or(PathBuf::from("."))
        .join("axis");
    let niri_layout = NiriLayoutProvider::new(config_dir);
    let set_border_color = Arc::new(SetBorderColorUseCase::new(niri_layout.clone()));

    wire_appearance(pres, set_border_color, &theme_svc, &wallpaper_svc);

    spawn_run_sync(pres);

    let lock_factory = LockScreenFactory::new();
    wire_lock_screen(
        pres,
        lock_factory,
        &wallpaper_svc,
        lock_gtk_handle,
        start_locked,
    );

    let bar_window = wire_bar(app, pres, uc);

    let qs_popup = wire_quick_settings(app, pres, uc);
    wire_notifications(app, pres, uc, &qs_popup);
    wire_osd(app, pres);

    let launcher_popup = wire_launcher(app, pres, &qs_popup);
    let agenda_popup = wire_agenda(app, pres, uc);
    let mpris_popup = wire_mpris(app, pres, p);

    let pp = pres.popup.clone();
    pp.add_popup(Box::new(qs_popup));
    pp.add_popup(Box::new(launcher_popup));
    pp.add_popup(Box::new(agenda_popup));
    pp.add_popup(Box::new(mpris_popup));

    let pp_sync = pp.clone();
    glib::spawn_future_local(async move {
        pp_sync.run_sync().await;
    });

    wire_auto_hide(pres, uc, &bar_window);
    wire_continuity_capture(app, pres, p);
    bar_window.present();
    wire_dbus_host(pres, uc, p, rt);
}

fn wire_continuity_capture(app: &libadwaita::Application, pres: &Presenters, p: &Providers) {
    let capture_controller = Rc::new(
        crate::widgets::continuity_capture::ContinuityCaptureController::new(
            app,
            p.continuity_sharing.clone(),
        ),
    );
    pres.continuity.add_view(Box::new(capture_controller));
}

fn wire_appearance(
    pres: &Presenters,
    set_border_color: Arc<SetBorderColorUseCase>,
    theme_svc: &Rc<ThemeService>,
    wallpaper_svc: &Rc<WallpaperService>,
) {
    let border_auto = set_border_color.clone();
    theme_svc.on_color_extracted(move |hex| {
        let uc = border_auto.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(hex).await {
                log::error!("[theme] border color sync failed: {e}");
            }
        });
    });

    pres.appearance.add_view(Box::new(theme_svc.clone()));
    pres.appearance.add_view(Box::new(wallpaper_svc.clone()));

    let border_manual = set_border_color.clone();
    pres.appearance.add_view(Box::new(FnView::new(
        move |status: &axis_domain::models::config::AppearanceConfig| {
            if let AccentColor::Custom(hex) = &status.accent_color {
                let uc = border_manual.clone();
                let hex_c = hex.clone();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(hex_c).await {
                        log::error!("[appearance] border color custom sync failed: {e}");
                    }
                });
            } else if status.accent_color != AccentColor::Auto {
                let uc = border_manual.clone();
                let hex = status.accent_color.hex_value().to_string();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(hex).await {
                        log::error!("[appearance] border color preset sync failed: {e}");
                    }
                });
            }
        },
    )));

    let app_sync = pres.appearance.clone();
    glib::spawn_future_local(async move {
        app_sync.run_sync().await;
    });
}

fn wire_lock_screen(
    pres: &Presenters,
    lock_factory: Rc<LockScreenFactory>,
    wallpaper_svc: &Rc<WallpaperService>,
    lock_gtk_handle: LockGtkHandle,
    start_locked: bool,
) {
    let lf_texture = lock_factory.clone();
    wallpaper_svc.on_texture_change(Rc::new(move |texture| {
        lf_texture.set_wallpaper(texture);
    }));

    let lp_auth = pres.lock.clone();
    let lf_auth = lock_factory.clone();
    lock_factory.on_authenticate(Rc::new(move |password| {
        let lf = lf_auth.clone();
        let lp = lp_auth.clone();
        lp_auth.authenticate(
            password,
            Rc::new(move |success| {
                lf.on_auth_result(success);
                if success {
                    lp.unlock();
                }
            }),
        );
    }));

    pres.lock.add_view(Box::new(lock_factory.clone()));
    pres.battery.add_view(Box::new(lock_factory.clone()));

    let lf = lock_factory.clone();
    lock_gtk_handle.set_content_factory(Box::new(move || lf.build_overlay()));

    if start_locked {
        log::info!("--locked flag detected, locking session");
        let lp = pres.lock.clone();
        glib::idle_add_local_once(move || {
            lp.lock();
        });
    }
}

fn wire_bar(app: &libadwaita::Application, pres: &Presenters, uc: &UseCases) -> BarWindow {
    let show_labels = uc
        .get_config
        .execute()
        .map(|c| c.bar.show_labels)
        .unwrap_or(true);

    let bar_window = BarWindow::new(app);
    bar_window.setup_content(pres, show_labels);
    bar_window
}

fn wire_quick_settings(
    app: &libadwaita::Application,
    pres: &Presenters,
    uc: &UseCases,
) -> QuickSettingsPopup {
    let qs_popup = QuickSettingsPopup::new(app);
    qs_popup.setup_audio(pres.audio.clone());
    qs_popup.setup_brightness(pres.brightness.clone());

    let power_actions = Rc::new(PowerActionStack::new(
        uc.suspend.clone(),
        uc.power_off.clone(),
        uc.reboot.clone(),
        uc.lock_session.clone(),
    ));
    qs_popup.setup_bottom_row(pres.battery.clone(), power_actions);

    qs_popup.setup_toggle(0, 0, pres.wifi_toggle.clone(), Some("wifi"));
    qs_popup.setup_toggle(0, 1, pres.bluetooth_toggle.clone(), Some("bluetooth"));
    qs_popup.setup_toggle(1, 0, pres.nightlight_toggle.clone(), Some("nightlight"));
    qs_popup.setup_toggle(1, 1, pres.dnd_toggle.clone(), None);
    qs_popup.setup_toggle(2, 0, pres.airplane_toggle.clone(), None);
    qs_popup.setup_toggle(2, 1, pres.continuity_toggle.clone(), None);
    qs_popup.setup_toggle(3, 0, pres.idle_inhibit_toggle.clone(), None);

    qs_popup.setup_wifi_sub_page(pres.network.clone());
    qs_popup.setup_bluetooth_sub_page(pres.bluetooth.clone());
    qs_popup.setup_audio_sub_page(pres.audio.clone());
    qs_popup.setup_nightlight_sub_page(pres.nightlight.clone());
    qs_popup
}

fn wire_notifications(
    app: &libadwaita::Application,
    pres: &Presenters,
    uc: &UseCases,
    qs_popup: &QuickSettingsPopup,
) {
    let np = pres.notification.clone();
    let on_close: Rc<dyn Fn(u32)> = Rc::new(move |id| {
        np.close_notification(id);
    });
    let np_act = pres.notification.clone();
    let on_action: Rc<dyn Fn(u32, String, Option<String>)> = Rc::new(move |id, key, user_input| {
        np_act.invoke_action(id, key, user_input);
    });

    let toast = Rc::new(NotificationToastManager::new(
        app,
        on_close.clone(),
        on_action.clone(),
    ));
    pres.notification.register_toast(toast.clone());
    pres.notification.add_view(Box::new(toast.clone()));

    {
        let dnd_presenter: Rc<Presenter<DndStatus>> =
            Rc::new(Presenter::from_subscribe_use_case(uc.subscribe_dnd.clone()));
        dnd_presenter.add_view(Box::new(toast.clone()));
        let dp = dnd_presenter.clone();
        glib::spawn_future_local(async move {
            dp.run_sync().await;
        });
    }

    let archive = Rc::new(NotificationArchive::new(
        on_close.clone(),
        on_action.clone(),
    ));
    qs_popup.setup_notification_archive(archive.container.clone());
    pres.notification.register_archive(archive.clone());
    pres.notification.add_view(Box::new(archive));

    qs_popup.set_notification_presenter(pres.notification.clone());
}

fn wire_osd(app: &libadwaita::Application, pres: &Presenters) {
    let osd = Rc::new(OsdManager::new(app));
    pres.audio.add_view(Box::new(osd.clone()));
    pres.brightness.add_view(Box::new(osd.clone()));
}

fn wire_launcher(
    app: &libadwaita::Application,
    pres: &Presenters,
    qs_popup: &QuickSettingsPopup,
) -> LauncherPopup {
    let launcher_popup = LauncherPopup::new(app);
    let lp = pres.launcher.clone();
    lp.add_view(Box::new(launcher_popup.clone()));

    let lp_search = lp.clone();
    launcher_popup.on_search(Box::new(move |query| {
        lp_search.search(query);
    }));

    let lp_nav = lp.clone();
    launcher_popup.on_select_next(Box::new(move || {
        lp_nav.select_next();
    }));

    let lp_nav2 = lp.clone();
    launcher_popup.on_select_prev(Box::new(move || {
        lp_nav2.select_prev();
    }));

    let lp_act = lp.clone();
    launcher_popup.on_activate(Box::new(move |idx| {
        lp_act.activate(idx);
    }));

    lp.on_close(make_popup_toggle(
        pres.toggle_popup.clone(),
        PopupType::Launcher,
        "launcher close",
    ));
    launcher_popup.on_escape(make_popup_toggle(
        pres.toggle_popup.clone(),
        PopupType::Launcher,
        "launcher escape",
    ));
    qs_popup.on_escape(make_popup_toggle(
        pres.toggle_popup.clone(),
        PopupType::QuickSettings,
        "QS escape",
    ));

    launcher_popup
}

fn wire_agenda(app: &libadwaita::Application, pres: &Presenters, uc: &UseCases) -> AgendaPopup {
    let agenda_popup = AgendaPopup::new(app);
    let ap = pres.agenda.clone();
    let agenda_popup_c = agenda_popup.clone();

    let ap_bind = ap.clone();
    glib::spawn_future_local(async move {
        ap_bind.bind(Box::new(agenda_popup_c)).await;
    });

    let ap_sync = ap.clone();
    let subscribe_popups = uc.subscribe_popups.clone();
    glib::spawn_future_local(async move {
        ap_sync.run_sync(subscribe_popups).await;
    });

    agenda_popup
}

fn wire_mpris(app: &libadwaita::Application, pres: &Presenters, p: &Providers) -> MprisPopup {
    let mpris_popup = MprisPopup::new(app);
    pres.mpris.add_view(Box::new(mpris_popup.clone()));

    let mp_pp = pres.mpris.clone();
    mpris_popup.on_play_pause(Box::new(move || {
        if let Some(id) = mp_pp.active_player_id() {
            mp_pp.play_pause(&id);
        }
    }));

    let mp_nx = pres.mpris.clone();
    mpris_popup.on_next(Box::new(move || {
        if let Some(id) = mp_nx.active_player_id() {
            mp_nx.next(&id);
        }
    }));

    let mp_pv = pres.mpris.clone();
    mpris_popup.on_previous(Box::new(move || {
        if let Some(id) = mp_pv.active_player_id() {
            mp_pv.previous(&id);
        }
    }));

    mpris_popup.on_escape(make_popup_toggle(
        pres.toggle_popup.clone(),
        PopupType::Mpris,
        "MPRIS escape",
    ));

    if let Some(ref dbus) = p.mpris_dbus {
        let dbus_clone = dbus.clone();
        mpris_popup.on_visibility_change(Box::new(move |visible| {
            dbus_clone.set_position_polling(visible);
        }));

        let pos_popup = mpris_popup.clone();
        let mut pos_rx = dbus.subscribe_positions();
        glib::spawn_future_local(async move {
            loop {
                if pos_rx.changed().await.is_err() {
                    break;
                }
                let (id, pos, len) = pos_rx.borrow().clone();
                if !id.is_empty() {
                    pos_popup.update_position(&id, pos, len);
                }
            }
        });
    }

    mpris_popup
}

fn wire_auto_hide(pres: &Presenters, uc: &UseCases, bar_window: &BarWindow) {
    let ahp = pres.auto_hide.clone();
    let bar_win = bar_window.clone();
    let subscribe_popups = uc.subscribe_popups.clone();
    glib::spawn_future_local(async move {
        if let Ok(mut stream) = subscribe_popups.execute().await {
            while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                ahp.set_force_visible(&bar_win, status.active_popup.is_some());
            }
        }
    });
}

fn wire_dbus_host(pres: &Presenters, uc: &UseCases, p: &Providers, rt: &tokio::runtime::Runtime) {
    let dbus_tp = pres.toggle_popup.clone();
    let dbus_lock_uc = uc.lock_session.clone();
    let cont_cmd_tx = p.continuity_service.cmd_tx();
    let cont_status_rx = p.continuity_service.snapshot_rx();
    rt.spawn(async move {
        dbus_host::run_dbus_host(
            {
                let tp = dbus_tp.clone();
                move || {
                    let tp = tp.clone();
                    tokio::spawn(async move {
                        if let Err(e) = tp.execute(PopupType::Launcher).await {
                            log::error!("[dbus-host] toggle launcher failed: {e}");
                        }
                    });
                }
            },
            {
                let uc = dbus_lock_uc.clone();
                move || {
                    let uc = uc.clone();
                    tokio::spawn(async move {
                        if let Err(e) = uc.execute().await {
                            log::error!("[dbus-host] lock failed: {e}");
                        }
                    });
                }
            },
            cont_cmd_tx,
            cont_status_rx,
        )
        .await;
    });
}

fn spawn_run_sync(pres: &Presenters) {
    macro_rules! spawn {
        ($field:ident) => {
            let p = pres.$field.clone();
            glib::spawn_future_local(async move { p.run_sync().await });
        };
    }
    spawn!(clock);
    spawn!(workspace);
    spawn!(audio);
    spawn!(brightness);
    spawn!(battery);
    spawn!(network);
    spawn!(bluetooth);
    spawn!(nightlight);
    spawn!(tray);
    spawn!(lock);
    spawn!(continuity);
    spawn!(mpris);
    spawn!(dnd_status);
    spawn!(airplane_status);
    spawn!(idle_inhibit_status);
    spawn!(notification);
}

fn make_popup_toggle(
    tp: Arc<TogglePopupUseCase>,
    pt: PopupType,
    label: &'static str,
) -> Box<dyn Fn() + 'static> {
    Box::new(move || {
        let tp = tp.clone();
        tokio::spawn(async move {
            if let Err(e) = tp.execute(pt).await {
                log::error!("[popup] {label} failed: {e}");
            }
        });
    })
}
