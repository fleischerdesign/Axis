use super::use_cases::UseCases;
use crate::presentation::agenda::{AgendaPresenter, AgendaPresenterArgs};
use crate::presentation::appearance::AppearancePresenter;
use crate::presentation::audio::{AudioPresenter, AudioPresenterArgs};
use crate::presentation::auto_hide::AutoHidePresenter;
use crate::presentation::battery::BatteryPresenter;
use crate::presentation::bluetooth::{BluetoothPresenter, BluetoothPresenterArgs};
use crate::presentation::brightness::BrightnessPresenter;
use crate::presentation::clock::ClockPresenter;
use crate::presentation::continuity::ContinuityPresenter;
use crate::presentation::launcher::LauncherPresenter;
use crate::presentation::lock::{LockPresenter, LockPresenterArgs};
use crate::presentation::mpris::{MprisPresenter, MprisPresenterArgs};
use crate::presentation::network::{NetworkPresenter, NetworkPresenterArgs};
use crate::presentation::nightlight::{NightlightPresenter, NightlightPresenterArgs};
use crate::presentation::notifications::{NotificationPresenter, NotificationPresenterArgs};
use crate::presentation::popups::PopupPresenter;
use crate::presentation::toggle::TogglePresenter;
use crate::presentation::tray::{TrayPresenter, TrayPresenterArgs};
use crate::presentation::workspaces::WorkspacePresenter;
use axis_application::use_cases::popups::TogglePopupUseCase;
use axis_application::use_cases::workspaces::toggle_overview::ToggleOverviewUseCase;
use axis_domain::models::airplane::AirplaneStatus;
use axis_domain::models::dnd::DndStatus;
use axis_domain::models::idle_inhibit::IdleInhibitStatus;
use axis_presentation::Presenter;
use futures_util::StreamExt;
use std::rc::Rc;
use std::sync::Arc;

pub struct Presenters {
    pub battery: Rc<BatteryPresenter>,
    pub clock: Rc<ClockPresenter>,
    pub workspace: Rc<WorkspacePresenter>,
    pub popup: Rc<PopupPresenter>,
    pub auto_hide: Rc<AutoHidePresenter>,
    pub audio: Rc<AudioPresenter>,
    pub brightness: Rc<BrightnessPresenter>,
    pub agenda: Rc<AgendaPresenter>,
    pub launcher: Rc<LauncherPresenter>,
    pub notification: Rc<NotificationPresenter>,
    pub network: Rc<NetworkPresenter>,
    pub bluetooth: Rc<BluetoothPresenter>,
    pub nightlight: Rc<NightlightPresenter>,
    pub appearance: Rc<AppearancePresenter>,
    pub tray: Rc<TrayPresenter>,
    pub lock: Rc<LockPresenter>,
    pub continuity: Rc<ContinuityPresenter>,
    pub mpris: Rc<MprisPresenter>,
    pub wifi_toggle: Rc<TogglePresenter>,
    pub bluetooth_toggle: Rc<TogglePresenter>,
    pub nightlight_toggle: Rc<TogglePresenter>,
    pub dnd_toggle: Rc<TogglePresenter>,
    pub airplane_toggle: Rc<TogglePresenter>,
    pub continuity_toggle: Rc<TogglePresenter>,
    pub idle_inhibit_toggle: Rc<TogglePresenter>,
    pub dnd_status: Rc<Presenter<DndStatus>>,
    pub idle_inhibit_status: Rc<Presenter<IdleInhibitStatus>>,
    pub airplane_status: Rc<Presenter<AirplaneStatus>>,
    pub toggle_popup: Arc<TogglePopupUseCase>,
    pub toggle_overview: Arc<ToggleOverviewUseCase>,
}

/// Creates a `TogglePresenter` from a subscribe use case (to observe state)
/// and a setter use case (to toggle on user action).
///
/// Parameters:
/// - `$uc`       : the `UseCases` struct
/// - `$label`    : human-readable toggle name
/// - `$active`   : icon when ON
/// - `$inactive` : icon when OFF
/// - `$sub_field`: field on `UseCases` holding the `SubscribeUseCase`
/// - `$set_field`: field on `UseCases` holding the setter use case
/// - `$extract`  : field on the status struct to extract `bool` from
/// - `$log_tag`  : label for error log messages
macro_rules! toggle_presenter {
    ($uc:ident, $label:literal, $active:literal, $inactive:literal,
     $sub_field:ident, $set_field:ident, $extract:ident, $log_tag:literal) => {
        Rc::new(TogglePresenter::new(
            $label,
            $active,
            $inactive,
            {
                let sub = $uc.$sub_field.clone();
                move || {
                    let sub = sub.clone();
                    async move {
                        sub.execute().await.map(|s| s.map(|st| st.$extract))
                    }
                }
            },
            {
                let toggle = $uc.$set_field.clone();
                move |enabled| {
                    let toggle = toggle.clone();
                    tokio::spawn(async move {
                        if let Err(e) = toggle.execute(enabled).await {
                            log::error!("[toggle] {} set failed: {e}", $log_tag);
                        }
                    });
                }
            },
        ))
    };
}

pub fn setup(uc: &UseCases, rt: &tokio::runtime::Runtime) -> Presenters {
    let battery = Rc::new(BatteryPresenter::new(uc.subscribe_power.clone()));
    let clock = Rc::new(ClockPresenter::new(uc.subscribe_clock.clone()));
    let workspace = Rc::new(WorkspacePresenter::new(uc.subscribe_ws.clone()));
    let popup = Rc::new(PopupPresenter::new(uc.subscribe_popups.clone()));
    let auto_hide = Rc::new(AutoHidePresenter::new(1, 500));

    let audio = Rc::new(AudioPresenter::new(
        AudioPresenterArgs {
            subscribe_uc: uc.subscribe_audio.clone(),
            get_status_uc: uc.get_audio_status.clone(),
            set_volume_uc: uc.set_volume.clone(),
            set_default_sink_uc: uc.set_default_sink.clone(),
            set_default_source_uc: uc.set_default_source.clone(),
            set_sink_input_volume_uc: uc.set_sink_input_volume.clone(),
        },
        rt,
    ));

    let brightness = Rc::new(BrightnessPresenter::new(
        uc.subscribe_brightness.clone(),
        uc.set_brightness.clone(),
    ));

    let agenda = Rc::new(AgendaPresenter::new(AgendaPresenterArgs {
        sync_events_uc: uc.sync_events.clone(),
        sync_tasks_uc: uc.sync_tasks.clone(),
        toggle_task_uc: uc.toggle_task.clone(),
        delete_task_uc: uc.delete_task.clone(),
        create_task_uc: uc.create_task.clone(),
    }));

    let launcher_executor = Arc::new(
        axis_application::use_cases::launcher::execute::ExecuteLauncherActionUseCase::new(),
    );
    let launcher = Rc::new(LauncherPresenter::new(
        uc.search_launcher.clone(),
        launcher_executor,
    ));

    let notification = Rc::new(NotificationPresenter::new(
        NotificationPresenterArgs {
            subscribe_uc: uc.subscribe_notifications.clone(),
            get_status_uc: uc.get_notifications_status.clone(),
            close_uc: uc.close_notification.clone(),
            invoke_action_uc: uc.invoke_notification_action.clone(),
        },
        rt,
    ));

    let network = Rc::new(NetworkPresenter::new(
        NetworkPresenterArgs {
            subscribe_uc: uc.subscribe_network.clone(),
            get_status_uc: uc.get_network_status.clone(),
            connect_uc: uc.connect_to_ap.clone(),
            disconnect_uc: uc.disconnect_wifi.clone(),
            start_scan_uc: uc.scan_wifi.clone(),
        },
        rt,
    ));

    let bluetooth = Rc::new(BluetoothPresenter::new(
        BluetoothPresenterArgs {
            subscribe_uc: uc.subscribe_bluetooth.clone(),
            get_status_uc: uc.get_bluetooth_status.clone(),
            connect_uc: uc.bt_connect.clone(),
            disconnect_uc: uc.bt_disconnect.clone(),
            start_scan_uc: uc.bt_start_scan.clone(),
            stop_scan_uc: uc.bt_stop_scan.clone(),
        },
        rt,
    ));

    let nightlight = Rc::new(NightlightPresenter::new(
        NightlightPresenterArgs {
            subscribe_uc: uc.subscribe_nightlight.clone(),
            get_status_uc: uc.get_nightlight_status.clone(),
            set_enabled_uc: uc.nl_set_enabled.clone(),
            set_temp_day_uc: uc.nl_set_temp_day.clone(),
            set_temp_night_uc: uc.nl_set_temp_night.clone(),
            set_schedule_uc: uc.nl_set_schedule.clone(),
        },
        rt,
    ));

    let appearance = Rc::new(AppearancePresenter::new(
        uc.subscribe_appearance.clone(),
        uc.get_appearance_status.clone(),
        rt,
    ));

    let tray = Rc::new(TrayPresenter::new(
        TrayPresenterArgs {
            subscribe_uc: uc.subscribe_tray.clone(),
            get_status_uc: uc.get_tray_status.clone(),
            activate_uc: uc.tray_activate.clone(),
            context_menu_uc: uc.tray_context_menu.clone(),
            scroll_uc: uc.tray_scroll.clone(),
        },
        rt,
    ));

    let lock = Rc::new(LockPresenter::new(LockPresenterArgs {
        subscribe_uc: uc.subscribe_lock.clone(),
        lock_uc: uc.lock_session.clone(),
        unlock_uc: uc.unlock_session.clone(),
        authenticate_uc: uc.authenticate.clone(),
    }));

    let continuity = Rc::new(ContinuityPresenter::new(
        uc.subscribe_continuity.clone(),
        uc.get_continuity_status.clone(),
        rt,
    ));

    let mpris = Rc::new(MprisPresenter::new(
        MprisPresenterArgs {
            subscribe_uc: uc.subscribe_mpris.clone(),
            get_status_uc: uc.get_mpris_status.clone(),
            play_pause_uc: uc.mpris_play_pause.clone(),
            next_uc: uc.mpris_next.clone(),
            previous_uc: uc.mpris_previous.clone(),
        },
        rt,
    ));

    let wifi_toggle = toggle_presenter!(uc, "Wi-Fi",
        "network-wireless-signal-excellent-symbolic",
        "network-wireless-offline-symbolic",
        subscribe_network, set_wifi, is_wifi_enabled, "wifi");

    let bluetooth_toggle = toggle_presenter!(uc, "Bluetooth",
        "bluetooth-active-symbolic",
        "bluetooth-disabled-symbolic",
        subscribe_bluetooth, bt_set_powered, powered, "bluetooth");

    let nightlight_toggle = toggle_presenter!(uc, "Nightlight",
        "weather-clear-night-symbolic",
        "weather-clear-night-symbolic",
        subscribe_nightlight, nl_set_enabled, enabled, "nightlight");

    let dnd_status = Rc::new(Presenter::from_subscribe_use_case(uc.subscribe_dnd.clone()));
    let idle_inhibit_status = Rc::new(Presenter::from_subscribe_use_case(
        uc.subscribe_idle_inhibit.clone(),
    ));
    let airplane_status = Rc::new(Presenter::from_subscribe_use_case(
        uc.subscribe_airplane.clone(),
    ));

    let dnd_toggle = toggle_presenter!(uc, "DND",
        "preferences-system-notifications-symbolic",
        "notifications-disabled-symbolic",
        subscribe_dnd, dnd_set_enabled, enabled, "dnd");

    let airplane_toggle = toggle_presenter!(uc, "Airplane",
        "airplane-mode-symbolic",
        "airplane-mode-symbolic",
        subscribe_airplane, ap_set_enabled, enabled, "airplane");

    let continuity_toggle = toggle_presenter!(uc, "Continuity",
        "input-mouse-symbolic",
        "input-mouse-symbolic",
        subscribe_continuity, continuity_set_enabled, enabled, "continuity");

    let idle_inhibit_toggle = toggle_presenter!(uc, "Idle Inhibit",
        "changes-prevent-symbolic",
        "changes-allow-symbolic",
        subscribe_idle_inhibit, idle_inhibit_set, inhibited, "idle inhibit");

    Presenters {
        battery,
        clock,
        workspace,
        popup,
        auto_hide,
        audio,
        brightness,
        agenda,
        launcher,
        notification,
        network,
        bluetooth,
        nightlight,
        appearance,
        tray,
        lock,
        continuity,
        mpris,
        wifi_toggle,
        bluetooth_toggle,
        nightlight_toggle,
        dnd_toggle,
        airplane_toggle,
        continuity_toggle,
        idle_inhibit_toggle,
        dnd_status,
        idle_inhibit_status,
        airplane_status,
        toggle_popup: uc.toggle_popup.clone(),
        toggle_overview: uc.toggle_overview.clone(),
    }
}
