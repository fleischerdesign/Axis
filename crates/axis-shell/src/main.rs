use libadwaita::prelude::*;
use futures_util::StreamExt;
use gtk4::{glib, gdk};
use clap::Parser;

use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::power::suspend::SuspendUseCase;
use axis_application::use_cases::power::power_off::PowerOffUseCase;
use axis_application::use_cases::power::reboot::RebootUseCase;
use axis_application::use_cases::lock::lock::LockSessionUseCase;
use axis_application::use_cases::lock::unlock::UnlockSessionUseCase;
use axis_application::use_cases::lock::authenticate::AuthenticateUseCase;
use axis_application::use_cases::audio::set_volume::SetVolumeUseCase;
use axis_application::use_cases::audio::set_default_sink::SetDefaultSinkUseCase;
use axis_application::use_cases::audio::set_default_source::SetDefaultSourceUseCase;
use axis_application::use_cases::audio::set_sink_input_volume::SetSinkInputVolumeUseCase;
use axis_application::use_cases::workspaces::focus::FocusWorkspaceUseCase;
use axis_application::use_cases::popups::TogglePopupUseCase;
use axis_application::use_cases::brightness::set::SetBrightnessUseCase;
use axis_application::use_cases::launcher::search::SearchLauncherUseCase;
use axis_application::use_cases::network::connect_to_ap::ConnectToApUseCase;
use axis_application::use_cases::network::disconnect_wifi::DisconnectWifiUseCase;
use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::set_powered::SetBluetoothPoweredUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;
use axis_application::use_cases::nightlight::set_enabled::SetNightlightEnabledUseCase;
use axis_application::use_cases::nightlight::set_temp_day::SetNightlightTempDayUseCase;
use axis_application::use_cases::nightlight::set_temp_night::SetNightlightTempNightUseCase;
use axis_application::use_cases::nightlight::set_schedule::SetNightlightScheduleUseCase;
use axis_application::use_cases::tray::activate::ActivateTrayItemUseCase;
use axis_application::use_cases::tray::context_menu::ContextMenuTrayItemUseCase;
use axis_application::use_cases::tray::scroll::ScrollTrayItemUseCase;
use axis_application::use_cases::notifications::close_notification::CloseNotificationUseCase;
use axis_application::use_cases::notifications::invoke_action::InvokeNotificationActionUseCase;
use axis_application::use_cases::notifications::show_notification::ShowNotificationUseCase;
use axis_application::use_cases::layout::set_border::SetBorderColorUseCase;
use axis_application::use_cases::continuity::set_enabled::SetContinuityEnabledUseCase;
use axis_application::use_cases::continuity::confirm_pin::ConfirmPinUseCase;
use axis_application::use_cases::continuity::reject_pin::RejectPinUseCase;

use axis_domain::models::appearance::{AccentColor, ColorScheme};
use axis_domain::models::config::AxisConfig;
use axis_domain::ports::config::ConfigProvider;
use axis_domain::ports::network::NetworkProvider;
use axis_domain::ports::lock::LockProvider;
use axis_domain::ports::nightlight::NightlightProvider;
use axis_domain::ports::dnd::DndProvider;
use axis_domain::ports::airplane::AirplaneProvider;
use axis_domain::ports::power::PowerProvider;
use axis_domain::ports::audio::AudioProvider;
use axis_domain::ports::workspaces::WorkspaceProvider;
use axis_domain::ports::brightness::BrightnessProvider;
use axis_domain::ports::appearance::AppearanceProvider;
use axis_domain::ports::clock::ClockProvider;
use axis_domain::ports::bluetooth::BluetoothProvider;
use axis_domain::ports::popups::PopupProvider;
use axis_domain::ports::notifications::NotificationProvider;
use axis_domain::ports::tray::TrayProvider;
use axis_domain::ports::continuity::ContinuityProvider;
use axis_domain::models::dnd::DndStatus;

use axis_infrastructure::adapters::google_auth::GoogleCloudAuthProvider;
use axis_infrastructure::mocks::clock::MockClockProvider;
use axis_infrastructure::adapters::power::LogindPowerProvider;
use axis_infrastructure::adapters::workspaces::NiriWorkspaceProvider;
use axis_infrastructure::adapters::popups::LocalPopupProvider;
use axis_infrastructure::adapters::pulse::PulseAudioProvider;
use axis_infrastructure::adapters::backlight::SysfsBrightnessProvider;
use axis_infrastructure::adapters::network::NetworkManagerProvider;
use axis_infrastructure::adapters::bluetooth::BlueZProvider;
use axis_infrastructure::adapters::nightlight::ConfigNightlightProvider;
use axis_infrastructure::adapters::dnd::ConfigDndProvider;
use axis_infrastructure::adapters::launcher::CompositeLauncherProvider;
use axis_infrastructure::adapters::notifications::ZbusNotificationProvider;
use axis_infrastructure::adapters::appearance::ConfigAppearanceProvider;
use axis_infrastructure::adapters::config::FileConfigProvider;
use axis_infrastructure::adapters::airplane::ConfigAirplaneProvider;
use axis_infrastructure::adapters::tray::StatusNotifierTrayProvider;
use axis_infrastructure::adapters::niri_layout::NiriLayoutProvider;
use axis_infrastructure::adapters::continuity::ContinuityService;
use std::path::PathBuf;
use std::sync::Arc;
use std::rc::Rc;

mod presentation;
mod widgets;
mod utils;
mod services;

use widgets::agenda::AgendaPopup;
use presentation::agenda::AgendaPresenter;
use widgets::bar_window::BarWindow;
use widgets::quick_settings::QuickSettingsPopup;
use widgets::launcher_popup::LauncherPopup;
use widgets::notification_toast::NotificationToastManager;
use widgets::notification_archive::NotificationArchive;
use widgets::continuity_capture::ContinuityCaptureController;
use widgets::osd::OsdManager;
use widgets::components::power_actions::PowerActionStack;
use widgets::lock_screen::LockScreenFactory;
use presentation::battery::BatteryPresenter;
use presentation::clock::ClockPresenter;
use presentation::audio::AudioPresenter;
use presentation::workspaces::WorkspacePresenter;
use presentation::auto_hide::AutoHidePresenter;
use presentation::popups::PopupPresenter;
use presentation::toggle::TogglePresenter;
use presentation::brightness::BrightnessPresenter;
use presentation::launcher::LauncherPresenter;
use presentation::notifications::NotificationPresenter;
use axis_presentation::{Presenter, view::FnView};
use presentation::network::NetworkPresenter;
use presentation::bluetooth::BluetoothPresenter;
use presentation::nightlight::NightlightPresenter;
use presentation::appearance::AppearancePresenter;
use presentation::lock::LockPresenter;
use presentation::continuity::ContinuityPresenter;
use presentation::tray::TrayPresenter;

use services::theme_service::ThemeService;
use services::wallpaper_service::WallpaperService;

use axis_infrastructure::adapters::lock::SessionLockProvider;

fn main() -> glib::ExitCode {
    setup_logger().expect("Failed to initialize logger");
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    let cli = Cli::parse();

    let cli_config = AxisConfig {
        appearance: axis_domain::models::config::AppearanceConfig {
            wallpaper: cli.wallpaper,
            accent_color: cli.accent.as_deref().map(parse_accent).unwrap_or_default(),
            color_scheme: cli.mode.as_deref().and_then(parse_color_scheme).unwrap_or_default(),
            font: cli.font,
        },
        ..AxisConfig::default()
    };

    let start_locked = cli.locked;

    let prog_name = std::env::args().next().unwrap_or_else(|| "axis-shell".to_string());

    let app = libadwaita::Application::builder()
        .application_id("org.axis.shell")
        .build();

    let theme_provider: Rc<std::cell::OnceCell<Rc<gtk4::CssProvider>>> = Rc::new(std::cell::OnceCell::new());
    let theme_provider_for_startup = theme_provider.clone();

    app.connect_startup(move |_| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let theme_css = Rc::new(gtk4::CssProvider::new());
        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
            &*theme_css,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
        let _ = theme_provider_for_startup.set(theme_css);
    });

    let power_provider: Arc<dyn PowerProvider> = rt.block_on(async {
        match LogindPowerProvider::new().await {
            Ok(p) => p as Arc<dyn PowerProvider>,
            Err(_) => {
                log::warn!("[power] logind/UPower not available, using empty provider");
                axis_infrastructure::mocks::power::MockPowerProvider::new()
            }
        }
    });
    let audio_provider: Arc<dyn AudioProvider> = rt.block_on(async {
        match PulseAudioProvider::new().await {
            Ok(p) => p as Arc<dyn AudioProvider>,
            Err(_) => {
                log::warn!("[audio] PulseAudio not available, using empty provider");
                axis_infrastructure::mocks::audio::MockAudioProvider::new()
            }
        }
    });
    let workspace_provider: Arc<dyn WorkspaceProvider> = rt.block_on(async {
        match NiriWorkspaceProvider::new().await {
            Ok(p) => p as Arc<dyn WorkspaceProvider>,
            Err(_) => {
                log::warn!("[workspaces] Niri IPC not available, using empty provider");
                axis_infrastructure::mocks::workspaces::MockWorkspaceProvider::new()
            }
        }
    });
    let brightness_provider: Arc<dyn BrightnessProvider> = rt.block_on(async {
        match SysfsBrightnessProvider::new().await {
            Ok(p) => p as Arc<dyn BrightnessProvider>,
            Err(_) => {
                log::warn!("[brightness] sysfs backlight not available, using empty provider");
                axis_infrastructure::mocks::brightness::MockBrightnessProvider::new()
            }
        }
    });
    let network_provider: Arc<dyn NetworkProvider> = rt.block_on(async {
        match NetworkManagerProvider::new().await {
            Ok(p) => p as Arc<dyn NetworkProvider>,
            Err(_) => {
                log::warn!("[network] NetworkManager not available, using empty provider");
                axis_infrastructure::mocks::network::MockNetworkProvider::new()
            }
        }
    });
    let bluetooth_provider: Arc<dyn BluetoothProvider> = rt.block_on(async {
        match BlueZProvider::new().await {
            Ok(p) => p as Arc<dyn BluetoothProvider>,
            Err(_) => {
                log::warn!("[bluetooth] BlueZ not available, using empty provider");
                axis_infrastructure::mocks::bluetooth::MockBluetoothProvider::new()
            }
        }
    });
    let config_provider = FileConfigProvider::new(cli_config);
    let nightlight_provider: Arc<dyn NightlightProvider> = rt.block_on(ConfigNightlightProvider::new(config_provider.clone()));
    let airplane_provider: Arc<dyn AirplaneProvider> = rt.block_on(ConfigAirplaneProvider::new(config_provider.clone()));
    let appearance_provider: Arc<dyn AppearanceProvider> = rt.block_on(ConfigAppearanceProvider::new(config_provider.clone()));
    let dnd_provider: Arc<dyn DndProvider> = rt.block_on(ConfigDndProvider::new(config_provider.clone()));
    let clock_provider: Arc<dyn ClockProvider> = MockClockProvider::new();
    let popup_provider: Arc<dyn PopupProvider> = LocalPopupProvider::new();
    let launcher_provider = CompositeLauncherProvider::new();
    let config_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("axis");
    let google_auth = GoogleCloudAuthProvider::new(config_dir.clone());

    let notification_provider: Arc<dyn NotificationProvider> = rt.block_on(async {
        match ZbusNotificationProvider::new().await {
            Ok(p) => p as Arc<dyn NotificationProvider>,
            Err(e) => {
                log::warn!("[notifications] Failed to register D-Bus service: {e}, using mock");
                axis_infrastructure::mocks::notifications::MockNotificationProvider::new()
            }
        }
    });
    let tray_provider: Arc<dyn TrayProvider> = rt.block_on(async {
        match StatusNotifierTrayProvider::new().await {
            Ok(p) => p as Arc<dyn TrayProvider>,
            Err(e) => {
                log::warn!("[tray] Failed to register StatusNotifierWatcher: {e}, using mock");
                axis_infrastructure::mocks::tray::MockTrayProvider::new()
            }
        }
    });

    let continuity_service = ContinuityService::new();
    let continuity_provider: Arc<dyn ContinuityProvider> = continuity_service.clone();

    let subscribe_power = Arc::new(SubscribeUseCase::new(power_provider.clone()));
    let suspend_uc = Arc::new(SuspendUseCase::new(power_provider.clone()));
    let power_off_uc = Arc::new(PowerOffUseCase::new(power_provider.clone()));
    let reboot_uc = Arc::new(RebootUseCase::new(power_provider.clone()));
    let (lock_provider_arc, lock_gtk_handle) = SessionLockProvider::new();
    let lock_provider: Arc<dyn LockProvider> = lock_provider_arc;
    let subscribe_lock = Arc::new(SubscribeUseCase::new(lock_provider.clone()));
    let lock_session_uc = Arc::new(LockSessionUseCase::new(lock_provider.clone()));
    let unlock_session_uc = Arc::new(UnlockSessionUseCase::new(lock_provider.clone()));
    let authenticate_uc = Arc::new(AuthenticateUseCase::new(lock_provider.clone()));
    let subscribe_clock = Arc::new(SubscribeUseCase::new(clock_provider.clone()));
    let subscribe_audio = Arc::new(SubscribeUseCase::new(audio_provider.clone()));
    let get_audio_status = Arc::new(GetStatusUseCase::new(audio_provider.clone()));
    let set_volume = Arc::new(SetVolumeUseCase::new(audio_provider.clone()));
    let subscribe_ws = Arc::new(SubscribeUseCase::new(workspace_provider.clone()));
    let focus_ws = Arc::new(FocusWorkspaceUseCase::new(workspace_provider.clone()));
    let subscribe_popups = Arc::new(SubscribeUseCase::new(popup_provider.clone()));
    let subscribe_popups_for_presenter = subscribe_popups.clone();
    let toggle_popup = Arc::new(TogglePopupUseCase::new(popup_provider.clone()));
    let subscribe_brightness = Arc::new(SubscribeUseCase::new(brightness_provider.clone()));
    let set_brightness = Arc::new(SetBrightnessUseCase::new(brightness_provider.clone()));
    let search_launcher = Arc::new(SearchLauncherUseCase::new(launcher_provider.clone()));

    let subscribe_network = Arc::new(SubscribeUseCase::new(network_provider.clone()));
    let subscribe_network_for_toggle = subscribe_network.clone();
    let get_network_status = Arc::new(GetStatusUseCase::new(network_provider.clone()));
    let connect_to_ap = Arc::new(ConnectToApUseCase::new(network_provider.clone()));
    let disconnect_wifi = Arc::new(DisconnectWifiUseCase::new(network_provider.clone()));
    let set_wifi = Arc::new(axis_application::use_cases::network::set_wifi::SetWifiEnabledUseCase::new(network_provider.clone()));

    let subscribe_bluetooth = Arc::new(SubscribeUseCase::new(bluetooth_provider.clone()));
    let subscribe_bluetooth_for_toggle = subscribe_bluetooth.clone();
    let get_bluetooth_status = Arc::new(GetStatusUseCase::new(bluetooth_provider.clone()));
    let bt_connect = Arc::new(ConnectBluetoothDeviceUseCase::new(bluetooth_provider.clone()));
    let bt_disconnect = Arc::new(DisconnectBluetoothDeviceUseCase::new(bluetooth_provider.clone()));
    let bt_set_powered = Arc::new(SetBluetoothPoweredUseCase::new(bluetooth_provider.clone()));
    let bt_start_scan = Arc::new(StartBluetoothScanUseCase::new(bluetooth_provider.clone()));
    let bt_stop_scan = Arc::new(StopBluetoothScanUseCase::new(bluetooth_provider.clone()));

    let subscribe_nightlight = Arc::new(SubscribeUseCase::new(nightlight_provider.clone()));
    let subscribe_nightlight_for_toggle = subscribe_nightlight.clone();
    let get_nightlight_status = Arc::new(GetStatusUseCase::new(nightlight_provider.clone()));
    let nl_set_enabled = Arc::new(SetNightlightEnabledUseCase::new(nightlight_provider.clone()));
    let nl_set_enabled_for_toggle = nl_set_enabled.clone();
    let nl_set_temp_day = Arc::new(SetNightlightTempDayUseCase::new(nightlight_provider.clone()));
    let nl_set_temp_night = Arc::new(SetNightlightTempNightUseCase::new(nightlight_provider.clone()));
    let nl_set_schedule = Arc::new(SetNightlightScheduleUseCase::new(nightlight_provider.clone()));

    let subscribe_dnd = Arc::new(SubscribeUseCase::new(dnd_provider.clone()));
    let dnd_set_enabled_uc = Arc::new(axis_application::use_cases::dnd::set_enabled::SetDndEnabledUseCase::new(dnd_provider.clone()));
    let subscribe_airplane = Arc::new(SubscribeUseCase::new(airplane_provider.clone()));
    let ap_set_enabled_uc = Arc::new(axis_application::use_cases::airplane::set_enabled::SetAirplaneModeUseCase::new(airplane_provider.clone()));

    let subscribe_continuity = Arc::new(SubscribeUseCase::new(continuity_provider.clone()));
    let subscribe_continuity_for_toggle = subscribe_continuity.clone();
    let get_continuity_status = Arc::new(GetStatusUseCase::new(continuity_provider.clone()));
    let continuity_set_enabled = Arc::new(SetContinuityEnabledUseCase::new(continuity_provider.clone()));
    let continuity_confirm_pin = Arc::new(ConfirmPinUseCase::new(continuity_provider.clone()));
    let continuity_reject_pin = Arc::new(RejectPinUseCase::new(continuity_provider.clone()));

    let subscribe_appearance = Arc::new(SubscribeUseCase::new(appearance_provider.clone()));
    let get_appearance_status = Arc::new(GetStatusUseCase::new(appearance_provider.clone()));

    let get_config_uc = Arc::new(axis_application::use_cases::config::get::GetConfigUseCase::new(config_provider.clone()));

    let subscribe_tray = Arc::new(SubscribeUseCase::new(tray_provider.clone()));
    let get_tray_status = Arc::new(GetStatusUseCase::new(tray_provider.clone()));
    let tray_activate = Arc::new(ActivateTrayItemUseCase::new(tray_provider.clone()));
    let tray_context_menu = Arc::new(ContextMenuTrayItemUseCase::new(tray_provider.clone()));
    let tray_scroll = Arc::new(ScrollTrayItemUseCase::new(tray_provider.clone()));

    let battery_presenter = Arc::new(BatteryPresenter::new(subscribe_power));
    let clock_presenter = Arc::new(ClockPresenter::new(subscribe_clock));
    let workspace_presenter = Arc::new(WorkspacePresenter::new(subscribe_ws, focus_ws));
    let popup_presenter = Arc::new(PopupPresenter::new(subscribe_popups_for_presenter));
    let auto_hide_presenter = Arc::new(AutoHidePresenter::new(1, 500));
    let audio_presenter = Rc::new(AudioPresenter::new(
        subscribe_audio, get_audio_status, set_volume,
        Arc::new(SetDefaultSinkUseCase::new(audio_provider.clone())),
        Arc::new(SetDefaultSourceUseCase::new(audio_provider.clone())),
        Arc::new(SetSinkInputVolumeUseCase::new(audio_provider.clone())),
        &rt,
    ));
    let brightness_presenter = Rc::new(BrightnessPresenter::new(subscribe_brightness, set_brightness));
    
    let google_calendar = axis_infrastructure::adapters::google_calendar::GoogleCalendarProvider::new(google_auth.clone());
    let google_tasks = axis_infrastructure::adapters::google_tasks::GoogleTasksProvider::new(google_auth.clone());
    
    let sync_calendar_uc = Arc::new(axis_application::use_cases::cloud::sync_calendar::SyncCalendarUseCase::new(google_calendar));
    let sync_tasks_uc = Arc::new(axis_application::use_cases::cloud::sync_tasks::SyncTasksUseCase::new(google_tasks.clone()));
    let toggle_task_uc = Arc::new(axis_application::use_cases::tasks::toggle_task::ToggleTaskUseCase::new(google_tasks.clone()));
    let delete_task_uc = Arc::new(axis_application::use_cases::tasks::delete_task::DeleteTaskUseCase::new(google_tasks.clone()));
    let create_task_uc = Arc::new(axis_application::use_cases::tasks::create_task::CreateTaskUseCase::new(google_tasks));
    
    let agenda_presenter = Rc::new(AgendaPresenter::new(sync_calendar_uc, sync_tasks_uc, toggle_task_uc, delete_task_uc, create_task_uc));

    let subscribe_notifications = Arc::new(SubscribeUseCase::new(notification_provider.clone()));
    let get_notifications_status = Arc::new(GetStatusUseCase::new(notification_provider.clone()));
    let close_notification_uc = Arc::new(CloseNotificationUseCase::new(notification_provider.clone()));
    let invoke_notification_action_uc = Arc::new(InvokeNotificationActionUseCase::new(notification_provider.clone()));
    let show_notification_uc = Arc::new(ShowNotificationUseCase::new(notification_provider.clone()));

    subscribe_continuity_notifications(
        continuity_provider.clone(),
        show_notification_uc.clone(),
        continuity_confirm_pin.clone(),
        continuity_reject_pin.clone(),
        &rt,
    );

    let config_cp: Arc<dyn ConfigProvider> = config_provider.clone();
    wire_continuity_sync(config_cp, continuity_provider.clone(), &rt);

    let launcher_presenter = Rc::new(LauncherPresenter::new(search_launcher));
    let notification_presenter = Rc::new(NotificationPresenter::new(
        subscribe_notifications, get_notifications_status,
        close_notification_uc, invoke_notification_action_uc, &rt,
    ));

    let network_presenter = Rc::new(NetworkPresenter::new(
        subscribe_network, get_network_status, connect_to_ap, disconnect_wifi, &rt,
    ));
    let bluetooth_full_presenter = Rc::new(BluetoothPresenter::new(
        subscribe_bluetooth, get_bluetooth_status, bt_connect, bt_disconnect,
        bt_start_scan, bt_stop_scan, &rt,
    ));
    let nightlight_full_presenter = Rc::new(NightlightPresenter::new(
        subscribe_nightlight, get_nightlight_status, nl_set_enabled,
        nl_set_temp_day, nl_set_temp_night, nl_set_schedule, &rt,
    ));

    let appearance_presenter = Rc::new(AppearancePresenter::new(
        subscribe_appearance, get_appearance_status, &rt,
    ));

    let tray_presenter = Rc::new(TrayPresenter::new(
        subscribe_tray, get_tray_status, tray_activate, tray_context_menu, tray_scroll, &rt,
    ));

    let lock_presenter = Rc::new(LockPresenter::new(
        subscribe_lock, lock_session_uc.clone(), unlock_session_uc.clone(), authenticate_uc.clone(),
    ));

    let continuity_presenter = Rc::new(ContinuityPresenter::new(
        subscribe_continuity, get_continuity_status, &rt,
    ));

    let wifi_presenter = Rc::new(TogglePresenter::new(
        "Wi-Fi",
        "network-wireless-signal-excellent-symbolic",
        "network-wireless-offline-symbolic",
        {
            let uc = subscribe_network_for_toggle.clone();
            move || {
                let uc = uc.clone();
                async move {
                    uc.execute().await.map(|s| s.map(|status| status.is_wifi_enabled))
                }
            }
        },
        {
            let uc = set_wifi.clone();
            move |enabled| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(enabled).await {
                        log::error!("[toggle] wifi set_enabled failed: {e}");
                    }
                });
            }
        },
    ));

    let bluetooth_toggle_presenter = Rc::new(TogglePresenter::new(
        "Bluetooth",
        "bluetooth-active-symbolic",
        "bluetooth-disabled-symbolic",
        {
            let uc = subscribe_bluetooth_for_toggle.clone();
            move || {
                let uc = uc.clone();
                async move {
                    uc.execute().await.map(|s| s.map(|status| status.powered))
                }
            }
        },
        {
            let uc = bt_set_powered.clone();
            move |enabled| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(enabled).await {
                        log::error!("[toggle] bluetooth set_powered failed: {e}");
                    }
                });
            }
        },
    ));

    let nightlight_toggle_presenter = Rc::new(TogglePresenter::new(
        "Nightlight",
        "weather-clear-night-symbolic",
        "weather-clear-night-symbolic",
        {
            let uc = subscribe_nightlight_for_toggle.clone();
            move || {
                let uc = uc.clone();
                async move {
                    uc.execute().await.map(|s| s.map(|status| status.enabled))
                }
            }
        },
        {
            let uc = nl_set_enabled_for_toggle.clone();
            move |enabled| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(enabled).await {
                        log::error!("[toggle] nightlight set_enabled failed: {e}");
                    }
                });
            }
        },
    ));

    let dnd_presenter = Rc::new(TogglePresenter::new(
        "DND",
        "preferences-system-notifications-symbolic",
        "notifications-disabled-symbolic",
        {
            let uc = subscribe_dnd.clone();
            move || {
                let uc = uc.clone();
                async move {
                    uc.execute().await.map(|s| s.map(|status| status.enabled))
                }
            }
        },
        {
            let uc = dnd_set_enabled_uc.clone();
            move |enabled| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(enabled).await {
                        log::error!("[toggle] dnd set_enabled failed: {e}");
                    }
                });
            }
        },
    ));

    let airplane_presenter = Rc::new(TogglePresenter::new(
        "Airplane",
        "airplane-mode-symbolic",
        "airplane-mode-symbolic",
        {
            let uc = subscribe_airplane.clone();
            move || {
                let uc = uc.clone();
                async move {
                    uc.execute().await.map(|s| s.map(|status| status.enabled))
                }
            }
        },
        {
            let uc = ap_set_enabled_uc.clone();
            move |enabled| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(enabled).await {
                        log::error!("[toggle] airplane set_enabled failed: {e}");
                    }
                });
            }
        },
    ));

    let continuity_toggle_presenter = Rc::new(TogglePresenter::new(
        "Continuity",
        "input-mouse-symbolic",
        "input-mouse-symbolic",
        {
            let uc = subscribe_continuity_for_toggle.clone();
            move || {
                let uc = uc.clone();
                async move {
                    uc.execute().await.map(|s| s.map(|status| status.enabled))
                }
            }
        },
        {
            let uc = continuity_set_enabled.clone();
            move |enabled| {
                let uc = uc.clone();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(enabled).await {
                        log::error!("[toggle] continuity set_enabled failed: {e}");
                    }
                });
            }
        },
    ));

    let ap_sync = audio_presenter.clone();
    glib::spawn_future_local(async move { ap_sync.run_sync().await; });

    let bp_sync = brightness_presenter.clone();
    glib::spawn_future_local(async move { bp_sync.run_sync().await; });

    let bat_sync = battery_presenter.clone();
    glib::spawn_future_local(async move { bat_sync.run_sync().await; });

    let np_sync = network_presenter.clone();
    glib::spawn_future_local(async move { np_sync.run_sync().await; });

    let bt_sync = bluetooth_full_presenter.clone();
    glib::spawn_future_local(async move { bt_sync.run_sync().await; });

    let nl_sync = nightlight_full_presenter.clone();
    glib::spawn_future_local(async move { nl_sync.run_sync().await; });

    let tray_sync = tray_presenter.clone();
    glib::spawn_future_local(async move { tray_sync.run_sync().await; });

    let lock_sync = lock_presenter.clone();
    glib::spawn_future_local(async move { lock_sync.run_sync().await; });

    let cont_sync = continuity_presenter.clone();
    glib::spawn_future_local(async move { cont_sync.run_sync().await; });

    let dbus_tp = toggle_popup.clone();
    let dbus_lock_uc = lock_session_uc.clone();
    let cont_cmd_tx = continuity_service.cmd_tx();
    let cont_status_rx = continuity_service.snapshot_rx();
    rt.spawn(async move {
        services::dbus_host::run_dbus_host(
            {
                let tp = dbus_tp.clone();
                move || {
                    let tp = tp.clone();
                    tokio::spawn(async move {
                        if let Err(e) = tp.execute(axis_domain::models::popups::PopupType::Launcher).await {
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
        ).await;
    });

    let show_labels = get_config_uc.execute().map(|c| c.bar.show_labels).unwrap_or(true);

    let lock_gtk_handle_for_activate = lock_gtk_handle;
    app.connect_activate(move |app| {
        let theme_css = theme_provider.get().cloned().unwrap_or_else(|| {
            log::error!("theme provider not initialized, falling back to empty CSS");
            Rc::new(gtk4::CssProvider::new())
        });
        let theme_svc = Rc::new(ThemeService::new(theme_css));
        let wallpaper_svc = Rc::new(WallpaperService::new(app));
        let config_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("axis");
        let niri_layout = NiriLayoutProvider::new(config_dir);
        let set_border_color = Arc::new(SetBorderColorUseCase::new(niri_layout.clone()));

        let border_auto = set_border_color.clone();
        theme_svc.on_color_extracted(move |hex| {
            let uc = border_auto.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(hex).await {
                    log::error!("[theme] border color sync failed: {e}");
                }
            });
        });

        appearance_presenter.add_view(Box::new(theme_svc));
        appearance_presenter.add_view(Box::new(wallpaper_svc.clone()));

        let border_manual = set_border_color.clone();
        appearance_presenter.add_view(Box::new(FnView::new(move |status: &axis_domain::models::config::AppearanceConfig| {
            if let axis_domain::models::appearance::AccentColor::Custom(hex) = &status.accent_color {
                let uc = border_manual.clone();
                let hex_c = hex.clone();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(hex_c).await {
                        log::error!("[appearance] border color custom sync failed: {e}");
                    }
                });
            } else if status.accent_color != axis_domain::models::appearance::AccentColor::Auto {
                let uc = border_manual.clone();
                let hex = status.accent_color.hex_value().to_string();
                tokio::spawn(async move {
                    if let Err(e) = uc.execute(hex).await {
                        log::error!("[appearance] border color preset sync failed: {e}");
                    }
                });
            }
        })));

        let app_sync = appearance_presenter.clone();
        glib::spawn_future_local(async move { app_sync.run_sync().await; });

        let lock_factory = LockScreenFactory::new();

        let lf_texture = lock_factory.clone();
        wallpaper_svc.on_texture_change(Rc::new(move |texture| {
            lf_texture.set_wallpaper(texture);
        }));

        let lp_auth = lock_presenter.clone();
        let lf_auth = lock_factory.clone();
        lock_factory.on_authenticate(Rc::new(move |password| {
            let lf = lf_auth.clone();
            let lp = lp_auth.clone();
            lp_auth.authenticate(password, Rc::new(move |success| {
                lf.on_auth_result(success);
                if success {
                    lp.unlock();
                }
            }));
        }));

        lock_presenter.add_view(Box::new(lock_factory.clone()));
        battery_presenter.add_view(Box::new(lock_factory.clone()));

        let capture_controller = ContinuityCaptureController::new(app, continuity_provider.clone());
        continuity_presenter.add_view(Box::new(capture_controller));

        let lf = lock_factory.clone();
        lock_gtk_handle_for_activate.set_content_factory(Box::new(move || {
            lf.build_overlay()
        }));

        let bar_window = BarWindow::new(app);
        bar_window.setup_content(
            battery_presenter.clone(), clock_presenter.clone(), audio_presenter.clone(),
            workspace_presenter.clone(), auto_hide_presenter.clone(), tray_presenter.clone(),
            toggle_popup.clone(),
            show_labels,
        );

        let qs_popup = QuickSettingsPopup::new(app);
        qs_popup.setup_audio(audio_presenter.clone());
        qs_popup.setup_brightness(brightness_presenter.clone());

        let power_actions = Rc::new(PowerActionStack::new(
            suspend_uc.clone(), power_off_uc.clone(), reboot_uc.clone(), lock_session_uc.clone(),
        ));
        qs_popup.setup_bottom_row(battery_presenter.clone(), power_actions);

        qs_popup.setup_toggle(0, 0, wifi_presenter.clone(), Some("wifi"));
        qs_popup.setup_toggle(0, 1, bluetooth_toggle_presenter.clone(), Some("bluetooth"));
        qs_popup.setup_toggle(1, 0, nightlight_toggle_presenter.clone(), Some("nightlight"));
        qs_popup.setup_toggle(1, 1, dnd_presenter.clone(), None);
        qs_popup.setup_toggle(2, 0, airplane_presenter.clone(), None);
        qs_popup.setup_toggle(2, 1, continuity_toggle_presenter.clone(), None);

        qs_popup.setup_wifi_sub_page(network_presenter.clone());
        qs_popup.setup_bluetooth_sub_page(bluetooth_full_presenter.clone());
        qs_popup.setup_audio_sub_page(audio_presenter.clone());
        qs_popup.setup_nightlight_sub_page(nightlight_full_presenter.clone());

        let np = notification_presenter.clone();
        let on_close: std::rc::Rc<dyn Fn(u32)> = std::rc::Rc::new(move |id| {
            np.close_notification(id);
        });
        let np_act = notification_presenter.clone();
        let on_action: std::rc::Rc<dyn Fn(u32, String)> = std::rc::Rc::new(move |id, key| {
            np_act.invoke_action(id, key);
        });

        let toast = std::rc::Rc::new(NotificationToastManager::new(app, on_close.clone(), on_action.clone()));
        notification_presenter.register_toast(toast.clone());
        notification_presenter.add_view(Box::new(toast.clone()));

        {
            let subscribe_dnd_clone = subscribe_dnd.clone();
            let dnd_presenter: Rc<Presenter<DndStatus>> = Rc::new(Presenter::from_subscribe({
                let uc = subscribe_dnd_clone.clone();
                move || {
                    let uc = uc.clone();
                    async move { uc.execute().await }
                }
            }));
            dnd_presenter.add_view(Box::new(toast.clone()));
            let dp = dnd_presenter.clone();
            glib::spawn_future_local(async move { dp.run_sync().await; });

        }

        let archive = std::rc::Rc::new(NotificationArchive::new(on_close.clone(), on_action.clone()));
        qs_popup.setup_notification_archive(archive.container.clone());
        notification_presenter.register_archive(archive.clone());
        notification_presenter.add_view(Box::new(archive));

        qs_popup.set_notification_presenter(notification_presenter.clone());

        let osd = Rc::new(OsdManager::new(app));
        audio_presenter.add_view(Box::new(osd.clone()));
        brightness_presenter.add_view(Box::new(osd.clone()));

        let notif_sync = notification_presenter.clone();
        glib::spawn_future_local(async move { notif_sync.run_sync().await; });

        let launcher_popup = LauncherPopup::new(app);
        let lp = launcher_presenter.clone();
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

        let tp_close = toggle_popup.clone();
        lp.on_close(Box::new(move || {
            let tp = tp_close.clone();
            tokio::spawn(async move {
                if let Err(e) = tp.execute(axis_domain::models::popups::PopupType::Launcher).await {
                    log::error!("[popup] launcher close failed: {e}");
                }
            });
        }));

        let tp_esc = toggle_popup.clone();
        launcher_popup.on_escape(Box::new(move || {
            let tp = tp_esc.clone();
            tokio::spawn(async move {
                if let Err(e) = tp.execute(axis_domain::models::popups::PopupType::Launcher).await {
                    log::error!("[popup] launcher escape failed: {e}");
                }
            });
        }));

        let tp_esc_qs = toggle_popup.clone();
        qs_popup.on_escape(Box::new(move || {
            let tp = tp_esc_qs.clone();
            tokio::spawn(async move {
                if let Err(e) = tp.execute(axis_domain::models::popups::PopupType::QuickSettings).await {
                    log::error!("[popup] QS escape failed: {e}");
                }
            });
        }));

        let agenda_popup = AgendaPopup::new(app);
        let ap = agenda_presenter.clone();
        let agenda_popup_c = agenda_popup.clone();
        let subscribe_popups_clone = subscribe_popups.clone();

        let ap_bind = ap.clone();
        glib::spawn_future_local(async move {
            ap_bind.bind(Box::new(agenda_popup_c)).await;
        });

        let ap_sync = ap.clone();
        glib::spawn_future_local(async move {
            ap_sync.run_sync(subscribe_popups_clone).await;
        });

        let pp = popup_presenter.clone();
        pp.add_popup(Box::new(qs_popup));
        pp.add_popup(Box::new(launcher_popup));
        pp.add_popup(Box::new(agenda_popup));
        
        let pp_sync = pp.clone();
        glib::spawn_future_local(async move {
            pp_sync.run_sync().await;
        });

        let ahp = auto_hide_presenter.clone();
        let bar_win = bar_window.clone();
        let subscribe_popups_clone2 = subscribe_popups.clone();
        glib::spawn_future_local(async move {
            if let Ok(mut stream) = subscribe_popups_clone2.execute().await {
                while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                    ahp.set_force_visible(&bar_win, status.active_popup.is_some());
                }
        }
    });

        bar_window.present();

        if start_locked {
            log::info!("[main] --locked flag detected, locking session");
            let lp = lock_presenter.clone();
            glib::idle_add_local_once(move || {
                lp.lock();
            });
        }
    });

    app.run_with_args(&[&prog_name])
}

fn subscribe_continuity_notifications(
    continuity_provider: Arc<dyn ContinuityProvider>,
    show_notification_uc: Arc<ShowNotificationUseCase>,
    confirm_pin_uc: Arc<ConfirmPinUseCase>,
    reject_pin_uc: Arc<RejectPinUseCase>,
    rt: &tokio::runtime::Runtime,
) {
    use std::collections::HashMap;

    rt.spawn(async move {
        let mut stream = match continuity_provider.subscribe().await {
            Ok(s) => s,
            Err(e) => {
                log::error!("[continuity:notifications] Failed to subscribe: {e}");
                return;
            }
        };

        let mut last_notified: Option<String> = None;
        let mut last_connected: Option<String> = None;

        while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
            if let Some(pending) = &status.pending_pin {
                if pending.is_incoming {
                    let peer_id = &pending.peer_id;
                    if last_notified.as_deref() != Some(peer_id) {
                        last_notified = Some(peer_id.clone());

                        let notification = axis_domain::models::notifications::Notification {
                            id: u32::MAX - 2,
                            app_name: "Continuity".to_string(),
                            app_icon: "computer-symbolic".to_string(),
                            summary: "Gerätekopplung".to_string(),
                            body: format!(
                                "Kopplungsanfrage von {}\nPIN: {}",
                                pending.peer_name, pending.pin
                            ),
                            urgency: 2,
                            actions: vec![
                                axis_domain::models::notifications::NotificationAction {
                                    key: "accept".into(),
                                    label: "Bestätigen".into(),
                                },
                                axis_domain::models::notifications::NotificationAction {
                                    key: "reject".into(),
                                    label: "Ablehnen".into(),
                                },
                            ],
                            timeout: 0,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64,
                            internal_id: 0,
                            ignore_dnd: true,
                        };

                        let mut action_handlers: HashMap<
                            String,
                            axis_domain::ports::notifications::ActionHandler,
                        > = HashMap::new();

                        action_handlers.insert("accept".into(), Arc::new({
                            let uc = confirm_pin_uc.clone();
                            move || {
                                let uc = uc.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = uc.execute().await {
                                        log::error!("[continuity:notifications] confirm_pin failed: {e}");
                                    }
                                });
                            }
                        }));

                        action_handlers.insert("reject".into(), Arc::new({
                            let uc = reject_pin_uc.clone();
                            move || {
                                let uc = uc.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = uc.execute().await {
                                        log::error!("[continuity:notifications] reject_pin failed: {e}");
                                    }
                                });
                            }
                        }));

                        if let Err(e) = show_notification_uc.execute(notification, action_handlers).await {
                            log::error!("[continuity:notifications] show pairing notification failed: {e}");
                        }
                    }
                }
            } else {
                last_notified = None;
            }

            if let Some(conn) = &status.active_connection {
                if status.peer_configs.get(&conn.peer_id).is_some_and(|c| c.trusted) {
                    if last_connected.as_deref() != Some(&conn.peer_id) {
                        last_connected = Some(conn.peer_id.clone());

                        let notification = axis_domain::models::notifications::Notification {
                            id: u32::MAX - 3,
                            app_name: "Continuity".to_string(),
                            app_icon: "computer-symbolic".to_string(),
                            summary: "Verbunden".to_string(),
                            body: format!("Verbunden mit {}", conn.peer_name),
                            urgency: 1,
                            actions: vec![],
                            timeout: 5000,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64,
                            internal_id: 0,
                            ignore_dnd: false,
                        };

                        if let Err(e) = show_notification_uc.execute(notification, HashMap::new()).await {
                            log::error!("[continuity:notifications] show connected notification failed: {e}");
                        }
                    }
                } else {
                    last_connected = None;
                }
            } else {
                last_connected = None;
            }
        }
    });
}

fn wire_continuity_sync(
    config_provider: Arc<dyn ConfigProvider>,
    continuity_provider: Arc<dyn ContinuityProvider>,
    rt: &tokio::runtime::Runtime,
) {
    {
        let cont = continuity_provider.clone();
        let mut config_stream = match config_provider.subscribe() {
            Ok(s) => s,
            Err(e) => {
                log::error!("[continuity:sync] config subscribe failed: {e}");
                return;
            }
        };
        let mut last_enabled: Option<bool> = None;
        rt.spawn(async move {
            while let Some(config) = futures_util::StreamExt::next(&mut config_stream).await {
                let enabled = config.continuity.enabled;
                if last_enabled != Some(enabled) {
                    last_enabled = Some(enabled);
                    if let Err(e) = cont.set_enabled(enabled).await {
                        log::error!("[continuity:sync] config→continuity failed: {e}");
                    }
                }
            }
        });
    }

    {
        let cfg = config_provider.clone();
        let mut cont_stream = match rt.block_on(continuity_provider.subscribe()) {
            Ok(s) => s,
            Err(e) => {
                log::error!("[continuity:sync] continuity subscribe failed: {e}");
                return;
            }
        };
        let mut last_enabled: Option<bool> = None;
        rt.spawn(async move {
            while let Some(status) = futures_util::StreamExt::next(&mut cont_stream).await {
                let enabled = status.enabled;
                if last_enabled != Some(enabled) {
                    last_enabled = Some(enabled);
                    if let Err(e) = cfg.update(Box::new(move |c: &mut AxisConfig| {
                        c.continuity.enabled = enabled;
                    })) {
                        log::error!("[continuity:sync] continuity→config failed: {e}");
                    }
                }
            }
        });
    }
}

#[derive(clap::Parser)]
#[command(name = "axis-shell", about = "Axis Desktop Shell")]
struct Cli {
    #[arg(long)]
    wallpaper: Option<String>,

    #[arg(long)]
    locked: bool,

    #[arg(long)]
    accent: Option<String>,

    #[arg(long, value_name = "dark|light|system")]
    mode: Option<String>,

    #[arg(long)]
    font: Option<String>,
}

fn parse_accent(s: &str) -> AccentColor {
    match s.to_lowercase().as_str() {
        "blue" => AccentColor::Blue,
        "teal" => AccentColor::Teal,
        "green" => AccentColor::Green,
        "yellow" => AccentColor::Yellow,
        "orange" => AccentColor::Orange,
        "red" => AccentColor::Red,
        "pink" => AccentColor::Pink,
        "purple" => AccentColor::Purple,
        "auto" => AccentColor::Auto,
        _ => AccentColor::Custom(s.to_string()),
    }
}

fn parse_color_scheme(s: &str) -> Option<ColorScheme> {
    match s.to_lowercase().as_str() {
        "dark" => Some(ColorScheme::Dark),
        "light" => Some(ColorScheme::Light),
        "system" => Some(ColorScheme::System),
        _ => None,
    }
}

fn setup_logger() -> Result<(), fern::InitError> {
    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info);

    if let Ok(lvl) = std::env::var("RUST_LOG") {
        if let Ok(parsed) = lvl.parse() {
            dispatch = dispatch.level(parsed);
        }
    }

    dispatch.chain(std::io::stdout()).apply()?;
    Ok(())
}
