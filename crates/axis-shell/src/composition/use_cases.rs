use super::providers::Providers;
use axis_application::use_cases::audio::set_default_sink::SetDefaultSinkUseCase;
use axis_application::use_cases::audio::set_default_source::SetDefaultSourceUseCase;
use axis_application::use_cases::audio::set_sink_input_volume::SetSinkInputVolumeUseCase;
use axis_application::use_cases::audio::set_volume::SetVolumeUseCase;
use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::pair_accept::PairAcceptUseCase;
use axis_application::use_cases::bluetooth::pair_reject::PairRejectUseCase;
use axis_application::use_cases::bluetooth::set_powered::SetBluetoothPoweredUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;
use axis_application::use_cases::brightness::set::SetBrightnessUseCase;
use axis_application::use_cases::continuity::confirm_pin::ConfirmPinUseCase;
use axis_application::use_cases::continuity::reject_pin::RejectPinUseCase;
use axis_application::use_cases::continuity::set_enabled::SetContinuityEnabledUseCase;
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::idle_inhibit::set_inhibited::SetIdleInhibitUseCase;
use axis_application::use_cases::launcher::execute::ExecuteLauncherActionUseCase;
use axis_application::use_cases::launcher::search::SearchLauncherUseCase;
use axis_application::use_cases::lock::authenticate::AuthenticateUseCase;
use axis_application::use_cases::lock::session::LockSessionUseCase;
use axis_application::use_cases::lock::unlock::UnlockSessionUseCase;
use axis_application::use_cases::mpris::next::NextTrackUseCase;
use axis_application::use_cases::mpris::play_pause::PlayPauseUseCase;
use axis_application::use_cases::mpris::previous::PreviousTrackUseCase;
use axis_application::use_cases::network::connect_to_ap::ConnectToApUseCase;
use axis_application::use_cases::network::disconnect_wifi::DisconnectWifiUseCase;
use axis_application::use_cases::nightlight::set_enabled::SetNightlightEnabledUseCase;
use axis_application::use_cases::nightlight::set_schedule::SetNightlightScheduleUseCase;
use axis_application::use_cases::nightlight::set_temp_day::SetNightlightTempDayUseCase;
use axis_application::use_cases::nightlight::set_temp_night::SetNightlightTempNightUseCase;
use axis_application::use_cases::notifications::close_notification::CloseNotificationUseCase;
use axis_application::use_cases::notifications::invoke_action::InvokeNotificationActionUseCase;
use axis_application::use_cases::notifications::show_notification::ShowNotificationUseCase;
use axis_application::use_cases::popups::TogglePopupUseCase;
use axis_application::use_cases::power::power_off::PowerOffUseCase;
use axis_application::use_cases::power::reboot::RebootUseCase;
use axis_application::use_cases::power::suspend::SuspendUseCase;
use axis_application::use_cases::tray::activate::ActivateTrayItemUseCase;
use axis_application::use_cases::tray::context_menu::ContextMenuTrayItemUseCase;
use axis_application::use_cases::tray::scroll::ScrollTrayItemUseCase;
use axis_application::use_cases::workspaces::toggle_overview::ToggleOverviewUseCase;
use axis_domain::models::{
    airplane::AirplaneStatus, audio::AudioStatus, bluetooth::BluetoothStatus,
    brightness::BrightnessStatus, clock::ClockStatus, config::AppearanceConfig,
    continuity::ContinuityStatus, dnd::DndStatus, idle_inhibit::IdleInhibitStatus,
    lock::LockStatus, mpris::MprisStatus, network::NetworkStatus, nightlight::NightlightStatus,
    notifications::NotificationStatus, popups::PopupStatus, power::PowerStatus, tray::TrayStatus,
    workspaces::WorkspaceStatus,
};
use axis_domain::ports::{
    airplane::AirplaneProvider, appearance::AppearanceProvider, audio::AudioProvider,
    bluetooth::BluetoothProvider, brightness::BrightnessProvider, clock::ClockProvider,
    continuity::ContinuityProvider, dnd::DndProvider, idle_inhibit::IdleInhibitProvider,
    lock::LockProvider, mpris::MprisProvider, network::NetworkProvider,
    nightlight::NightlightProvider, notifications::NotificationProvider, popups::PopupProvider,
    power::PowerProvider, tray::TrayProvider, workspaces::WorkspaceProvider,
};
use std::sync::Arc;

pub struct UseCases {
    pub subscribe_power: Arc<SubscribeUseCase<dyn PowerProvider, PowerStatus>>,
    pub suspend: Arc<SuspendUseCase>,
    pub power_off: Arc<PowerOffUseCase>,
    pub reboot: Arc<RebootUseCase>,
    pub subscribe_lock: Arc<SubscribeUseCase<dyn LockProvider, LockStatus>>,
    pub lock_session: Arc<LockSessionUseCase>,
    pub unlock_session: Arc<UnlockSessionUseCase>,
    pub authenticate: Arc<AuthenticateUseCase>,
    pub subscribe_clock: Arc<SubscribeUseCase<dyn ClockProvider, ClockStatus>>,
    pub subscribe_audio: Arc<SubscribeUseCase<dyn AudioProvider, AudioStatus>>,
    pub get_audio_status: Arc<GetStatusUseCase<dyn AudioProvider, AudioStatus>>,
    pub set_volume: Arc<SetVolumeUseCase>,
    pub set_default_sink: Arc<SetDefaultSinkUseCase>,
    pub set_default_source: Arc<SetDefaultSourceUseCase>,
    pub set_sink_input_volume: Arc<SetSinkInputVolumeUseCase>,
    pub subscribe_ws: Arc<SubscribeUseCase<dyn WorkspaceProvider, WorkspaceStatus>>,
    pub toggle_overview: Arc<ToggleOverviewUseCase>,
    pub subscribe_popups: Arc<SubscribeUseCase<dyn PopupProvider, PopupStatus>>,
    pub toggle_popup: Arc<TogglePopupUseCase>,
    pub subscribe_brightness: Arc<SubscribeUseCase<dyn BrightnessProvider, BrightnessStatus>>,
    pub set_brightness: Arc<SetBrightnessUseCase>,
    pub search_launcher: Arc<SearchLauncherUseCase>,
    pub execute_launcher: Arc<ExecuteLauncherActionUseCase>,
    pub subscribe_network: Arc<SubscribeUseCase<dyn NetworkProvider, NetworkStatus>>,
    pub get_network_status: Arc<GetStatusUseCase<dyn NetworkProvider, NetworkStatus>>,
    pub connect_to_ap: Arc<ConnectToApUseCase>,
    pub disconnect_wifi: Arc<DisconnectWifiUseCase>,
    pub set_wifi: Arc<axis_application::use_cases::network::set_wifi::SetWifiEnabledUseCase>,
    pub subscribe_bluetooth: Arc<SubscribeUseCase<dyn BluetoothProvider, BluetoothStatus>>,
    pub get_bluetooth_status: Arc<GetStatusUseCase<dyn BluetoothProvider, BluetoothStatus>>,
    pub bt_connect: Arc<ConnectBluetoothDeviceUseCase>,
    pub bt_disconnect: Arc<DisconnectBluetoothDeviceUseCase>,
    pub bt_set_powered: Arc<SetBluetoothPoweredUseCase>,
    pub bt_start_scan: Arc<StartBluetoothScanUseCase>,
    pub bt_stop_scan: Arc<StopBluetoothScanUseCase>,
    pub bt_pair_accept: Arc<PairAcceptUseCase>,
    pub bt_pair_reject: Arc<PairRejectUseCase>,
    pub subscribe_nightlight: Arc<SubscribeUseCase<dyn NightlightProvider, NightlightStatus>>,
    pub get_nightlight_status: Arc<GetStatusUseCase<dyn NightlightProvider, NightlightStatus>>,
    pub nl_set_enabled: Arc<SetNightlightEnabledUseCase>,
    pub nl_set_temp_day: Arc<SetNightlightTempDayUseCase>,
    pub nl_set_temp_night: Arc<SetNightlightTempNightUseCase>,
    pub nl_set_schedule: Arc<SetNightlightScheduleUseCase>,
    pub subscribe_dnd: Arc<SubscribeUseCase<dyn DndProvider, DndStatus>>,
    pub dnd_set_enabled: Arc<axis_application::use_cases::dnd::set_enabled::SetDndEnabledUseCase>,
    pub subscribe_idle_inhibit: Arc<SubscribeUseCase<dyn IdleInhibitProvider, IdleInhibitStatus>>,
    pub idle_inhibit_set: Arc<SetIdleInhibitUseCase>,
    pub subscribe_airplane: Arc<SubscribeUseCase<dyn AirplaneProvider, AirplaneStatus>>,
    pub ap_set_enabled:
        Arc<axis_application::use_cases::airplane::set_enabled::SetAirplaneModeUseCase>,
    pub subscribe_continuity: Arc<SubscribeUseCase<dyn ContinuityProvider, ContinuityStatus>>,
    pub get_continuity_status: Arc<GetStatusUseCase<dyn ContinuityProvider, ContinuityStatus>>,
    pub continuity_set_enabled: Arc<SetContinuityEnabledUseCase>,
    pub continuity_confirm_pin: Arc<ConfirmPinUseCase>,
    pub continuity_reject_pin: Arc<RejectPinUseCase>,
    pub subscribe_appearance: Arc<SubscribeUseCase<dyn AppearanceProvider, AppearanceConfig>>,
    pub get_appearance_status: Arc<GetStatusUseCase<dyn AppearanceProvider, AppearanceConfig>>,
    pub get_config: Arc<axis_application::use_cases::config::get::GetConfigUseCase>,
    pub subscribe_tray: Arc<SubscribeUseCase<dyn TrayProvider, TrayStatus>>,
    pub get_tray_status: Arc<GetStatusUseCase<dyn TrayProvider, TrayStatus>>,
    pub tray_activate: Arc<ActivateTrayItemUseCase>,
    pub tray_context_menu: Arc<ContextMenuTrayItemUseCase>,
    pub tray_scroll: Arc<ScrollTrayItemUseCase>,
    pub subscribe_notifications:
        Arc<SubscribeUseCase<dyn NotificationProvider, NotificationStatus>>,
    pub get_notifications_status:
        Arc<GetStatusUseCase<dyn NotificationProvider, NotificationStatus>>,
    pub close_notification: Arc<CloseNotificationUseCase>,
    pub invoke_notification_action: Arc<InvokeNotificationActionUseCase>,
    pub show_notification: Arc<ShowNotificationUseCase>,
    pub sync_events: Arc<axis_application::use_cases::agenda::sync_events::SyncEventsUseCase>,
    pub sync_tasks: Arc<axis_application::use_cases::agenda::sync_tasks::SyncTasksUseCase>,
    pub toggle_task: Arc<axis_application::use_cases::agenda::toggle_task::ToggleTaskUseCase>,
    pub delete_task: Arc<axis_application::use_cases::agenda::delete_task::DeleteTaskUseCase>,
    pub create_task: Arc<axis_application::use_cases::agenda::create_task::CreateTaskUseCase>,
    pub subscribe_mpris: Arc<SubscribeUseCase<dyn MprisProvider, MprisStatus>>,
    pub get_mpris_status: Arc<GetStatusUseCase<dyn MprisProvider, MprisStatus>>,
    pub mpris_play_pause: Arc<PlayPauseUseCase>,
    pub mpris_next: Arc<NextTrackUseCase>,
    pub mpris_previous: Arc<PreviousTrackUseCase>,
}

pub fn setup(p: &Providers) -> UseCases {
    let subscribe_power = Arc::new(SubscribeUseCase::new(p.power.clone()));
    let suspend = Arc::new(SuspendUseCase::new(p.power.clone()));
    let power_off = Arc::new(PowerOffUseCase::new(p.power.clone()));
    let reboot = Arc::new(RebootUseCase::new(p.power.clone()));
    let subscribe_lock = Arc::new(SubscribeUseCase::new(p.lock.clone()));
    let lock_session = Arc::new(LockSessionUseCase::new(p.lock.clone()));
    let unlock_session = Arc::new(UnlockSessionUseCase::new(p.lock.clone()));
    let authenticate = Arc::new(AuthenticateUseCase::new(p.lock.clone()));
    let subscribe_clock = Arc::new(SubscribeUseCase::new(p.clock.clone()));
    let subscribe_audio = Arc::new(SubscribeUseCase::new(p.audio.clone()));
    let get_audio_status = Arc::new(GetStatusUseCase::new(p.audio.clone()));
    let set_volume = Arc::new(SetVolumeUseCase::new(p.audio.clone()));
    let set_default_sink = Arc::new(SetDefaultSinkUseCase::new(p.audio.clone()));
    let set_default_source = Arc::new(SetDefaultSourceUseCase::new(p.audio.clone()));
    let set_sink_input_volume = Arc::new(SetSinkInputVolumeUseCase::new(p.audio.clone()));
    let subscribe_ws = Arc::new(SubscribeUseCase::new(p.workspace.clone()));
    let toggle_overview = Arc::new(ToggleOverviewUseCase::new(p.workspace.clone()));
    let subscribe_popups = Arc::new(SubscribeUseCase::new(p.popup.clone()));
    let toggle_popup = Arc::new(TogglePopupUseCase::new(p.popup.clone()));
    let subscribe_brightness = Arc::new(SubscribeUseCase::new(p.brightness.clone()));
    let set_brightness = Arc::new(SetBrightnessUseCase::new(p.brightness.clone()));
    let search_launcher = Arc::new(SearchLauncherUseCase::new(p.launcher.clone()));
    let execute_launcher = Arc::new(ExecuteLauncherActionUseCase::new());
    let subscribe_network = Arc::new(SubscribeUseCase::new(p.network.clone()));
    let get_network_status = Arc::new(GetStatusUseCase::new(p.network.clone()));
    let connect_to_ap = Arc::new(ConnectToApUseCase::new(p.network.clone()));
    let disconnect_wifi = Arc::new(DisconnectWifiUseCase::new(p.network.clone()));
    let set_wifi = Arc::new(
        axis_application::use_cases::network::set_wifi::SetWifiEnabledUseCase::new(
            p.network.clone(),
        ),
    );
    let subscribe_bluetooth = Arc::new(SubscribeUseCase::new(p.bluetooth.clone()));
    let get_bluetooth_status = Arc::new(GetStatusUseCase::new(p.bluetooth.clone()));
    let bt_connect = Arc::new(ConnectBluetoothDeviceUseCase::new(p.bluetooth.clone()));
    let bt_disconnect = Arc::new(DisconnectBluetoothDeviceUseCase::new(p.bluetooth.clone()));
    let bt_set_powered = Arc::new(SetBluetoothPoweredUseCase::new(p.bluetooth.clone()));
    let bt_start_scan = Arc::new(StartBluetoothScanUseCase::new(p.bluetooth.clone()));
    let bt_stop_scan = Arc::new(StopBluetoothScanUseCase::new(p.bluetooth.clone()));
    let bt_pair_accept = Arc::new(PairAcceptUseCase::new(p.bluetooth.clone()));
    let bt_pair_reject = Arc::new(PairRejectUseCase::new(p.bluetooth.clone()));
    let subscribe_nightlight = Arc::new(SubscribeUseCase::new(p.nightlight.clone()));
    let get_nightlight_status = Arc::new(GetStatusUseCase::new(p.nightlight.clone()));
    let nl_set_enabled = Arc::new(SetNightlightEnabledUseCase::new(p.nightlight.clone()));
    let nl_set_temp_day = Arc::new(SetNightlightTempDayUseCase::new(p.nightlight.clone()));
    let nl_set_temp_night = Arc::new(SetNightlightTempNightUseCase::new(p.nightlight.clone()));
    let nl_set_schedule = Arc::new(SetNightlightScheduleUseCase::new(p.nightlight.clone()));
    let subscribe_dnd = Arc::new(SubscribeUseCase::new(p.dnd.clone()));
    let dnd_set_enabled = Arc::new(
        axis_application::use_cases::dnd::set_enabled::SetDndEnabledUseCase::new(p.dnd.clone()),
    );
    let subscribe_idle_inhibit = Arc::new(SubscribeUseCase::new(p.idle_inhibit.clone()));
    let idle_inhibit_set = Arc::new(SetIdleInhibitUseCase::new(p.idle_inhibit.clone()));
    let subscribe_airplane = Arc::new(SubscribeUseCase::new(p.airplane.clone()));
    let ap_set_enabled = Arc::new(
        axis_application::use_cases::airplane::set_enabled::SetAirplaneModeUseCase::new(
            p.airplane.clone(),
        ),
    );
    let subscribe_continuity = Arc::new(SubscribeUseCase::new(p.continuity.clone()));
    let get_continuity_status = Arc::new(GetStatusUseCase::new(p.continuity.clone()));
    let continuity_set_enabled = Arc::new(SetContinuityEnabledUseCase::new(p.continuity.clone()));
    let continuity_confirm_pin = Arc::new(ConfirmPinUseCase::new(p.continuity.clone()));
    let continuity_reject_pin = Arc::new(RejectPinUseCase::new(p.continuity.clone()));
    let subscribe_appearance = Arc::new(SubscribeUseCase::new(p.appearance.clone()));
    let get_appearance_status = Arc::new(GetStatusUseCase::new(p.appearance.clone()));
    let get_config =
        Arc::new(axis_application::use_cases::config::get::GetConfigUseCase::new(p.config.clone()));
    let subscribe_tray = Arc::new(SubscribeUseCase::new(p.tray.clone()));
    let get_tray_status = Arc::new(GetStatusUseCase::new(p.tray.clone()));
    let tray_activate = Arc::new(ActivateTrayItemUseCase::new(p.tray.clone()));
    let tray_context_menu = Arc::new(ContextMenuTrayItemUseCase::new(p.tray.clone()));
    let tray_scroll = Arc::new(ScrollTrayItemUseCase::new(p.tray.clone()));
    let subscribe_notifications = Arc::new(SubscribeUseCase::new(p.notification.clone()));
    let get_notifications_status = Arc::new(GetStatusUseCase::new(p.notification.clone()));
    let close_notification = Arc::new(CloseNotificationUseCase::new(p.notification.clone()));
    let invoke_notification_action =
        Arc::new(InvokeNotificationActionUseCase::new(p.notification.clone()));
    let show_notification = Arc::new(ShowNotificationUseCase::new(p.notification.clone()));
    let sync_events = Arc::new(
        axis_application::use_cases::agenda::sync_events::SyncEventsUseCase::new(
            p.google_agenda.clone(),
        ),
    );
    let sync_tasks = Arc::new(
        axis_application::use_cases::agenda::sync_tasks::SyncTasksUseCase::new(
            p.google_agenda.clone(),
        ),
    );
    let toggle_task = Arc::new(
        axis_application::use_cases::agenda::toggle_task::ToggleTaskUseCase::new(
            p.google_agenda.clone(),
        ),
    );
    let delete_task = Arc::new(
        axis_application::use_cases::agenda::delete_task::DeleteTaskUseCase::new(
            p.google_agenda.clone(),
        ),
    );
    let create_task = Arc::new(
        axis_application::use_cases::agenda::create_task::CreateTaskUseCase::new(
            p.google_agenda.clone(),
        ),
    );
    let subscribe_mpris = Arc::new(SubscribeUseCase::new(p.mpris.clone()));
    let get_mpris_status = Arc::new(GetStatusUseCase::new(p.mpris.clone()));
    let mpris_play_pause = Arc::new(PlayPauseUseCase::new(p.mpris.clone()));
    let mpris_next = Arc::new(NextTrackUseCase::new(p.mpris.clone()));
    let mpris_previous = Arc::new(PreviousTrackUseCase::new(p.mpris.clone()));
    UseCases {
        subscribe_power,
        suspend,
        power_off,
        reboot,
        subscribe_lock,
        lock_session,
        unlock_session,
        authenticate,
        subscribe_clock,
        subscribe_audio,
        get_audio_status,
        set_volume,
        set_default_sink,
        set_default_source,
        set_sink_input_volume,
        subscribe_ws,
        toggle_overview,
        subscribe_popups,
        toggle_popup,
        subscribe_brightness,
        set_brightness,
        search_launcher,
        execute_launcher,
        subscribe_network,
        get_network_status,
        connect_to_ap,
        disconnect_wifi,
        set_wifi,
        subscribe_bluetooth,
        get_bluetooth_status,
        bt_connect,
        bt_disconnect,
        bt_set_powered,
        bt_start_scan,
        bt_stop_scan,
        bt_pair_accept,
        bt_pair_reject,
        subscribe_nightlight,
        get_nightlight_status,
        nl_set_enabled,
        nl_set_temp_day,
        nl_set_temp_night,
        nl_set_schedule,
        subscribe_dnd,
        dnd_set_enabled,
        subscribe_idle_inhibit,
        idle_inhibit_set,
        subscribe_airplane,
        ap_set_enabled,
        subscribe_continuity,
        get_continuity_status,
        continuity_set_enabled,
        continuity_confirm_pin,
        continuity_reject_pin,
        subscribe_appearance,
        get_appearance_status,
        get_config,
        subscribe_tray,
        get_tray_status,
        tray_activate,
        tray_context_menu,
        tray_scroll,
        subscribe_notifications,
        get_notifications_status,
        close_notification,
        invoke_notification_action,
        show_notification,
        sync_events,
        sync_tasks,
        toggle_task,
        delete_task,
        create_task,
        subscribe_mpris,
        get_mpris_status,
        mpris_play_pause,
        mpris_next,
        mpris_previous,
    }
}
