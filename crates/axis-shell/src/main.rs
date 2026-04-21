use libadwaita::prelude::*;
use gtk4::{glib, gdk};
use clap::Parser;

use axis_application::use_cases::power::subscribe::SubscribeToPowerUpdatesUseCase;
use axis_application::use_cases::power::suspend::SuspendUseCase;
use axis_application::use_cases::power::power_off::PowerOffUseCase;
use axis_application::use_cases::power::reboot::RebootUseCase;
use axis_application::use_cases::lock::lock::LockSessionUseCase;
use axis_application::use_cases::lock::unlock::UnlockSessionUseCase;
use axis_application::use_cases::lock::authenticate::AuthenticateUseCase;
use axis_application::use_cases::lock::subscribe::SubscribeToLockUpdatesUseCase;
use axis_application::use_cases::clock::subscribe::SubscribeToClockUpdatesUseCase;
use axis_application::use_cases::audio::subscribe::SubscribeToAudioUpdatesUseCase;
use axis_application::use_cases::audio::get_status::GetAudioStatusUseCase;
use axis_application::use_cases::audio::set_volume::SetVolumeUseCase;
use axis_application::use_cases::audio::set_default_sink::SetDefaultSinkUseCase;
use axis_application::use_cases::audio::set_default_source::SetDefaultSourceUseCase;
use axis_application::use_cases::audio::set_sink_input_volume::SetSinkInputVolumeUseCase;
use axis_application::use_cases::workspaces::subscribe::SubscribeToWorkspaceUpdatesUseCase;
use axis_application::use_cases::workspaces::focus::FocusWorkspaceUseCase;
use axis_application::use_cases::popups::{SubscribeToPopupUpdatesUseCase, TogglePopupUseCase};
use axis_application::use_cases::brightness::subscribe::SubscribeToBrightnessUpdatesUseCase;
use axis_application::use_cases::brightness::set::SetBrightnessUseCase;
use axis_application::use_cases::launcher::search::SearchLauncherUseCase;

use axis_application::use_cases::network::subscribe::SubscribeToNetworkUpdatesUseCase;
use axis_application::use_cases::network::get_status::GetNetworkStatusUseCase;
use axis_application::use_cases::network::scan_wifi::ScanWifiUseCase;
use axis_application::use_cases::network::connect_to_ap::ConnectToApUseCase;
use axis_application::use_cases::network::disconnect_wifi::DisconnectWifiUseCase;

use axis_application::use_cases::bluetooth::subscribe::SubscribeToBluetoothUpdatesUseCase;
use axis_application::use_cases::bluetooth::get_status::GetBluetoothStatusUseCase;
use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::set_powered::SetBluetoothPoweredUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;

use axis_application::use_cases::nightlight::subscribe::SubscribeToNightlightUpdatesUseCase;
use axis_application::use_cases::nightlight::get_status::GetNightlightStatusUseCase;
use axis_application::use_cases::nightlight::set_enabled::SetNightlightEnabledUseCase;
use axis_application::use_cases::nightlight::set_temp_day::SetNightlightTempDayUseCase;
use axis_application::use_cases::nightlight::set_temp_night::SetNightlightTempNightUseCase;
use axis_application::use_cases::nightlight::set_schedule::SetNightlightScheduleUseCase;

use axis_application::use_cases::appearance::subscribe::SubscribeToAppearanceUseCase;
use axis_application::use_cases::appearance::get_status::GetAppearanceStatusUseCase;

use axis_application::use_cases::tray::subscribe::SubscribeToTrayUpdatesUseCase;
use axis_application::use_cases::tray::get_status::GetTrayStatusUseCase;
use axis_application::use_cases::tray::activate::ActivateTrayItemUseCase;
use axis_application::use_cases::tray::context_menu::ContextMenuTrayItemUseCase;
use axis_application::use_cases::tray::scroll::ScrollTrayItemUseCase;

use axis_domain::models::appearance::{AccentColor, ColorScheme};
use axis_domain::models::config::AxisConfig;

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
use axis_infrastructure::adapters::ipc::ZbusIpcProvider;
use axis_infrastructure::adapters::notifications::ZbusNotificationProvider;
use axis_infrastructure::adapters::appearance::ConfigAppearanceProvider;
use axis_infrastructure::adapters::config::FileConfigProvider;
use axis_infrastructure::adapters::airplane::ConfigAirplaneProvider;
use axis_infrastructure::adapters::tray::StatusNotifierAdapter;

use axis_domain::ports::network::NetworkProvider;
use axis_domain::ports::lock::LockProvider;
use axis_domain::ports::nightlight::NightlightProvider;
use axis_domain::ports::dnd::DndProvider;
use axis_domain::ports::airplane::AirplaneProvider;
use axis_domain::ports::ipc::IpcProvider;
use axis_domain::ports::notifications::NotificationService;
use axis_domain::ports::popups::PopupProvider;
use axis_domain::models::ipc::IpcCommand;
use axis_domain::models::dnd::DndStatus;

use std::sync::Arc;
use std::rc::Rc;

mod presentation;
mod widgets;
mod utils;
mod services;

use widgets::bar_window::BarWindow;
use widgets::quick_settings::QuickSettingsPopup;
use widgets::launcher_popup::LauncherPopup;
use widgets::notification_toast::NotificationToastManager;
use widgets::notification_archive::NotificationArchive;
use widgets::osd::OsdManager;
use widgets::components::power_actions::PowerActionStack;
use widgets::lock_screen::LockScreenFactory;
use presentation::battery::BatteryPresenter;
use presentation::clock::ClockPresenter;
use presentation::audio::AudioPresenter;
use presentation::workspaces::WorkspacePresenter;
use presentation::auto_hide::AutoHidePresenter;
use presentation::popups::{PopupPresenter, PopupView};
use presentation::toggle::TogglePresenter;
use presentation::brightness::BrightnessPresenter;
use presentation::launcher::{LauncherPresenter, LauncherView};
use presentation::notifications::NotificationPresenter;
use presentation::presenter::{Presenter, View};
use presentation::network::NetworkPresenter;
use presentation::bluetooth::BluetoothPresenter;
use presentation::nightlight::NightlightPresenter;
use presentation::appearance::AppearancePresenter;
use presentation::lock::LockPresenter;
use presentation::tray::TrayPresenter;

use services::theme_service::ThemeService;
use services::wallpaper_service::WallpaperService;

use axis_infrastructure::adapters::lock::SessionLockProvider;

fn main() -> glib::ExitCode {
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

    let power_provider = rt.block_on(async {
        LogindPowerProvider::new().await.expect("Failed to connect to UPower/login1")
    });
    let audio_provider = rt.block_on(async {
        PulseAudioProvider::new().await.expect("Failed to connect to PulseAudio")
    });
    let workspace_provider = rt.block_on(async {
        NiriWorkspaceProvider::new().await.expect("Failed to connect to Niri IPC")
    });
    let brightness_provider = rt.block_on(async {
        SysfsBrightnessProvider::new().await.expect("Failed to connect to Brightness")
    });
    let network_provider = rt.block_on(async {
        NetworkManagerProvider::new().await.expect("Failed to connect to NetworkManager")
    });
    let bluetooth_provider: Arc<dyn axis_domain::ports::bluetooth::BluetoothProvider> = rt.block_on(async {
        match BlueZProvider::new().await {
            Ok(p) => p as Arc<dyn axis_domain::ports::bluetooth::BluetoothProvider>,
            Err(_) => {
                log::warn!("[bluetooth] BlueZ not available, using empty provider");
                Arc::new(axis_infrastructure::mocks::bluetooth::MockBluetoothProvider::new())
            }
        }
    });
    let config_provider = FileConfigProvider::new(cli_config);
    let nightlight_provider: Arc<dyn NightlightProvider> = ConfigNightlightProvider::new(config_provider.clone());
    let airplane_provider: Arc<dyn AirplaneProvider> = ConfigAirplaneProvider::new(config_provider.clone());
    let appearance_provider = ConfigAppearanceProvider::new(config_provider.clone());
    let dnd_provider = ConfigDndProvider::new(config_provider.clone());
    let clock_provider = Arc::new(MockClockProvider::new());
    let popup_provider = LocalPopupProvider::new();
    let launcher_provider = CompositeLauncherProvider::new();
    let ipc_provider = ZbusIpcProvider::new();
    let notification_provider: Arc<dyn NotificationService> = rt.block_on(async {
        match ZbusNotificationProvider::new().await {
            Ok(p) => p as Arc<dyn NotificationService>,
            Err(e) => {
                log::warn!("[notifications] Failed to register D-Bus service: {e}, using mock");
                Arc::new(axis_infrastructure::mocks::notifications::MockNotificationService::new())
            }
        }
    });
    let tray_provider: Arc<dyn axis_domain::ports::tray::TrayProvider> = rt.block_on(async {
        match StatusNotifierAdapter::new().await {
            Ok(p) => p as Arc<dyn axis_domain::ports::tray::TrayProvider>,
            Err(e) => {
                log::warn!("[tray] Failed to register StatusNotifierWatcher: {e}, using mock");
                Arc::new(axis_infrastructure::mocks::tray::MockTrayProvider::new())
            }
        }
    });

    let subscribe_power = Arc::new(SubscribeToPowerUpdatesUseCase::new(power_provider.clone()));
    let suspend_uc = Arc::new(SuspendUseCase::new(power_provider.clone()));
    let power_off_uc = Arc::new(PowerOffUseCase::new(power_provider.clone()));
    let reboot_uc = Arc::new(RebootUseCase::new(power_provider.clone()));
    let (lock_provider_arc, lock_gtk_handle) = SessionLockProvider::new();
    let lock_provider: Arc<dyn LockProvider> = lock_provider_arc;
    let subscribe_lock = Arc::new(SubscribeToLockUpdatesUseCase::new(lock_provider.clone()));
    let lock_session_uc = Arc::new(LockSessionUseCase::new(lock_provider.clone()));
    let unlock_session_uc = Arc::new(UnlockSessionUseCase::new(lock_provider.clone()));
    let authenticate_uc = Arc::new(AuthenticateUseCase::new(lock_provider.clone()));
    let subscribe_clock = Arc::new(SubscribeToClockUpdatesUseCase::new(clock_provider.clone()));
    let subscribe_audio = Arc::new(SubscribeToAudioUpdatesUseCase::new(audio_provider.clone()));
    let get_audio_status = Arc::new(GetAudioStatusUseCase::new(audio_provider.clone()));
    let set_volume = Arc::new(SetVolumeUseCase::new(audio_provider.clone()));
    let subscribe_ws = Arc::new(SubscribeToWorkspaceUpdatesUseCase::new(workspace_provider.clone()));
    let focus_ws = Arc::new(FocusWorkspaceUseCase::new(workspace_provider.clone()));
    let subscribe_popups = Arc::new(SubscribeToPopupUpdatesUseCase::new(popup_provider.clone()));
    let toggle_popup = Arc::new(TogglePopupUseCase::new(popup_provider.clone()));
    let subscribe_brightness = Arc::new(SubscribeToBrightnessUpdatesUseCase::new(brightness_provider.clone()));
    let set_brightness = Arc::new(SetBrightnessUseCase::new(brightness_provider.clone()));
    let search_launcher = Arc::new(SearchLauncherUseCase::new(launcher_provider.clone()));

    let subscribe_network = Arc::new(SubscribeToNetworkUpdatesUseCase::new(network_provider.clone()));
    let get_network_status = Arc::new(GetNetworkStatusUseCase::new(network_provider.clone()));
    let scan_wifi = Arc::new(ScanWifiUseCase::new(network_provider.clone()));
    let connect_to_ap = Arc::new(ConnectToApUseCase::new(network_provider.clone()));
    let disconnect_wifi = Arc::new(DisconnectWifiUseCase::new(network_provider.clone()));

    let subscribe_bluetooth = Arc::new(SubscribeToBluetoothUpdatesUseCase::new(bluetooth_provider.clone()));
    let get_bluetooth_status = Arc::new(GetBluetoothStatusUseCase::new(bluetooth_provider.clone()));
    let bt_connect = Arc::new(ConnectBluetoothDeviceUseCase::new(bluetooth_provider.clone()));
    let bt_disconnect = Arc::new(DisconnectBluetoothDeviceUseCase::new(bluetooth_provider.clone()));
    let bt_set_powered = Arc::new(SetBluetoothPoweredUseCase::new(bluetooth_provider.clone()));
    let bt_start_scan = Arc::new(StartBluetoothScanUseCase::new(bluetooth_provider.clone()));
    let bt_stop_scan = Arc::new(StopBluetoothScanUseCase::new(bluetooth_provider.clone()));

    let subscribe_nightlight = Arc::new(SubscribeToNightlightUpdatesUseCase::new(nightlight_provider.clone()));
    let get_nightlight_status = Arc::new(GetNightlightStatusUseCase::new(nightlight_provider.clone()));
    let nl_set_enabled = Arc::new(SetNightlightEnabledUseCase::new(nightlight_provider.clone()));
    let nl_set_temp_day = Arc::new(SetNightlightTempDayUseCase::new(nightlight_provider.clone()));
    let nl_set_temp_night = Arc::new(SetNightlightTempNightUseCase::new(nightlight_provider.clone()));
    let nl_set_schedule = Arc::new(SetNightlightScheduleUseCase::new(nightlight_provider.clone()));

    let subscribe_appearance = Arc::new(SubscribeToAppearanceUseCase::new(appearance_provider.clone()));
    let get_appearance_status = Arc::new(GetAppearanceStatusUseCase::new(appearance_provider.clone()));

    let subscribe_tray = Arc::new(SubscribeToTrayUpdatesUseCase::new(tray_provider.clone()));
    let get_tray_status = Arc::new(GetTrayStatusUseCase::new(tray_provider.clone()));
    let tray_activate = Arc::new(ActivateTrayItemUseCase::new(tray_provider.clone()));
    let tray_context_menu = Arc::new(ContextMenuTrayItemUseCase::new(tray_provider.clone()));
    let tray_scroll = Arc::new(ScrollTrayItemUseCase::new(tray_provider.clone()));

    let battery_presenter = Arc::new(BatteryPresenter::new(subscribe_power));
    let clock_presenter = Arc::new(ClockPresenter::new(subscribe_clock));
    let workspace_presenter = Arc::new(WorkspacePresenter::new(subscribe_ws, focus_ws));
    let popup_presenter = Arc::new(PopupPresenter::new(subscribe_popups));
    let auto_hide_presenter = Arc::new(AutoHidePresenter::new(1, 500));
    let audio_presenter = Rc::new(AudioPresenter::new(
        subscribe_audio, get_audio_status, set_volume,
        Arc::new(SetDefaultSinkUseCase::new(audio_provider.clone())),
        Arc::new(SetDefaultSourceUseCase::new(audio_provider.clone())),
        Arc::new(SetSinkInputVolumeUseCase::new(audio_provider.clone())),
        &rt,
    ));
    let brightness_presenter = Rc::new(BrightnessPresenter::new(subscribe_brightness, set_brightness));
    let launcher_presenter = LauncherPresenter::new(search_launcher);
    let notification_presenter = Rc::new(NotificationPresenter::new(notification_provider.clone()));

    let network_presenter = Rc::new(NetworkPresenter::new(
        subscribe_network, get_network_status, scan_wifi, connect_to_ap, disconnect_wifi, &rt,
    ));
    let bluetooth_presenter_sub = Rc::new(BluetoothPresenter::new(
        subscribe_bluetooth, get_bluetooth_status, bt_connect, bt_disconnect,
        bt_set_powered, bt_start_scan, bt_stop_scan, &rt,
    ));
    let nightlight_presenter_sub = Rc::new(NightlightPresenter::new(
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

    let net_prov = network_provider.clone();
    let wifi_presenter = Rc::new(TogglePresenter::new(
        "Wi-Fi",
        "network-wireless-signal-excellent-symbolic",
        "network-wireless-offline-symbolic",
        move || {
            let net = net_prov.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = net.subscribe().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status.is_wifi_enabled;
                    }
                }
            })
        },
        move |enabled| {
            let net = network_provider.clone();
            tokio::spawn(async move {
                let _ = net.set_wifi_enabled(enabled).await;
            });
        },
    ));

    let bt_prov = bluetooth_provider.clone();
    let bluetooth_presenter = Rc::new(TogglePresenter::new(
        "Bluetooth",
        "bluetooth-active-symbolic",
        "bluetooth-disabled-symbolic",
        move || {
            let bt = bt_prov.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = bt.subscribe().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status.powered;
                    }
                }
            })
        },
        move |_enabled| {},
    ));

    let nl_prov = nightlight_provider.clone();
    let nightlight_presenter = Rc::new(TogglePresenter::new(
        "Nightlight",
        "weather-clear-night-symbolic",
        "weather-clear-night-symbolic",
        move || {
            let nl = nl_prov.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = nl.subscribe().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status.enabled;
                    }
                }
            })
        },
        move |enabled| {
            let nl = nightlight_provider.clone();
            tokio::spawn(async move {
                let _ = nl.set_enabled(enabled).await;
            });
        },
    ));

    let dnd_prov = dnd_provider.clone();
    let dnd_for_toast = dnd_provider.clone();
    let dnd_presenter = Rc::new(TogglePresenter::new(
        "DND",
        "preferences-system-notifications-symbolic",
        "notifications-disabled-symbolic",
        move || {
            let dnd = dnd_prov.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = dnd.subscribe().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status.enabled;
                    }
                }
            })
        },
        move |enabled| {
            let dnd = dnd_provider.clone();
            tokio::spawn(async move {
                let _ = dnd.set_enabled(enabled).await;
            });
        },
    ));

    let ap_prov = airplane_provider.clone();
    let airplane_presenter = Rc::new(TogglePresenter::new(
        "Airplane",
        "airplane-mode-symbolic",
        "airplane-mode-symbolic",
        move || {
            let ap = ap_prov.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = ap.subscribe().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status.enabled;
                    }
                }
            })
        },
        move |enabled| {
            let ap = airplane_provider.clone();
            tokio::spawn(async move {
                let _ = ap.set_enabled(enabled).await;
            });
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

    let bt_sync = bluetooth_presenter_sub.clone();
    glib::spawn_future_local(async move { bt_sync.run_sync().await; });

    let nl_sync = nightlight_presenter_sub.clone();
    glib::spawn_future_local(async move { nl_sync.run_sync().await; });

    let tray_sync = tray_presenter.clone();
    glib::spawn_future_local(async move { tray_sync.run_sync().await; });

    let lock_sync = lock_presenter.clone();
    glib::spawn_future_local(async move { lock_sync.run_sync().await; });

    let ipc_tp = toggle_popup.clone();
    let ipc_lock_uc = lock_session_uc.clone();
    let ipc = ipc_provider.clone();
    rt.spawn(async move {
        if let Err(e) = ipc.run(Box::new(move |cmd: IpcCommand| {
            match cmd {
                IpcCommand::ToggleLauncher => {
                    let tp = ipc_tp.clone();
                    tokio::spawn(async move {
                        let _ = tp.execute(axis_domain::models::popups::PopupType::Launcher).await;
                    });
                }
                IpcCommand::Lock => {
                    let uc = ipc_lock_uc.clone();
                    tokio::spawn(async move {
                        let _ = uc.execute().await;
                    });
                }
            }
        })).await {
            log::error!("[ipc] D-Bus server error: {e}");
        }
    });

    let lock_gtk_handle_for_activate = lock_gtk_handle;
    app.connect_activate(move |app| {
        let theme_svc = Rc::new(ThemeService::new(theme_provider.get().expect("theme provider not initialized").clone()));
        let wallpaper_svc = Rc::new(WallpaperService::new(app));

        appearance_presenter.add_view(Box::new(theme_svc));
        appearance_presenter.add_view(Box::new(wallpaper_svc.clone()));

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

        let lf = lock_factory.clone();
        lock_gtk_handle_for_activate.set_content_factory(Box::new(move || {
            lf.build_overlay()
        }));

        let bar_window = BarWindow::new(app);
        bar_window.setup_content(
            battery_presenter.clone(), clock_presenter.clone(), audio_presenter.clone(),
            workspace_presenter.clone(), auto_hide_presenter.clone(), tray_presenter.clone(),
            toggle_popup.clone(),
        );

        let qs_popup = QuickSettingsPopup::new(app);
        qs_popup.setup_audio(audio_presenter.clone());
        qs_popup.setup_brightness(brightness_presenter.clone());

        let power_actions = Rc::new(PowerActionStack::new(
            suspend_uc.clone(), power_off_uc.clone(), reboot_uc.clone(), lock_session_uc.clone(),
        ));
        qs_popup.setup_bottom_row(battery_presenter.clone(), power_actions);

        qs_popup.setup_toggle(0, 0, wifi_presenter.clone(), Some("wifi"));
        qs_popup.setup_toggle(0, 1, bluetooth_presenter.clone(), Some("bluetooth"));
        qs_popup.setup_toggle(1, 0, nightlight_presenter.clone(), Some("nightlight"));
        qs_popup.setup_toggle(1, 1, dnd_presenter.clone(), None);
        qs_popup.setup_toggle(2, 0, airplane_presenter.clone(), None);

        qs_popup.setup_wifi_sub_page(network_presenter.clone());
        qs_popup.setup_bluetooth_sub_page(bluetooth_presenter_sub.clone());
        qs_popup.setup_audio_sub_page(audio_presenter.clone());
        qs_popup.setup_nightlight_sub_page(nightlight_presenter_sub.clone());

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
            let dnd_prov_c = dnd_for_toast.clone();
            let dnd_presenter: Rc<Presenter<dyn View<DndStatus>, DndStatus>> = Rc::new(Presenter::new(move || {
                let dnd = dnd_prov_c.clone();
                Box::pin(async_stream::stream! {
                    if let Ok(mut stream) = dnd.subscribe().await {
                        while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                            yield status;
                        }
                    }
                })
            }));
            dnd_presenter.add_view(Box::new(toast.clone()));
            let dp = dnd_presenter.clone();
            glib::spawn_future_local(async move { dp.run().await; });
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
        let lp_view: Rc<dyn LauncherView> = Rc::new(launcher_popup.clone());
        lp.bind(lp_view);

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
                let _ = tp.execute(axis_domain::models::popups::PopupType::Launcher).await;
            });
        }));

        let tp_esc = toggle_popup.clone();
        launcher_popup.on_escape(Box::new(move || {
            let tp = tp_esc.clone();
            tokio::spawn(async move {
                let _ = tp.execute(axis_domain::models::popups::PopupType::Launcher).await;
            });
        }));

        let tp_esc_qs = toggle_popup.clone();
        qs_popup.on_escape(Box::new(move || {
            let tp = tp_esc_qs.clone();
            tokio::spawn(async move {
                let _ = tp.execute(axis_domain::models::popups::PopupType::QuickSettings).await;
            });
        }));

        let pp = popup_presenter.clone();
        glib::spawn_future_local(async move {
            let popups: Vec<Box<dyn PopupView>> = vec![
                Box::new(qs_popup),
                Box::new(launcher_popup),
            ];
            pp.bind(popups).await;
        });

        let ahp = auto_hide_presenter.clone();
        let bar_win = bar_window.clone();
        let pp = popup_provider.clone();
        glib::spawn_future_local(async move {
            if let Ok(mut stream) = pp.subscribe().await {
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
