use crate::{Cli, parse_accent, parse_color_scheme};
use axis_domain::models::config::AxisConfig;
use axis_domain::ports::airplane::AirplaneProvider;
use axis_domain::ports::appearance::AppearanceProvider;
use axis_domain::ports::audio::AudioProvider;
use axis_domain::ports::bluetooth::BluetoothProvider;
use axis_domain::ports::brightness::BrightnessProvider;
use axis_domain::ports::clock::ClockProvider;
use axis_domain::ports::config::ConfigProvider;
use axis_domain::ports::continuity::ContinuityProvider;
use axis_domain::ports::continuity::ContinuitySharingProvider;
use axis_domain::ports::dnd::DndProvider;
use axis_domain::ports::idle_inhibit::IdleInhibitProvider;
use axis_domain::ports::lock::LockProvider;
use axis_domain::ports::mpris::MprisProvider;
use axis_domain::ports::network::NetworkProvider;
use axis_domain::ports::nightlight::NightlightProvider;
use axis_domain::ports::notifications::NotificationProvider;
use axis_domain::ports::popups::PopupProvider;
use axis_domain::ports::power::PowerProvider;
use axis_domain::ports::tray::TrayProvider;
use axis_domain::ports::workspaces::WorkspaceProvider;
use axis_infrastructure::adapters::airplane::ConfigAirplaneProvider;
use axis_infrastructure::adapters::appearance::ConfigAppearanceProvider;
use axis_infrastructure::adapters::backlight::SysfsBrightnessProvider;
use axis_infrastructure::adapters::bluetooth::BlueZProvider;
use axis_infrastructure::adapters::clock::SystemClockProvider;
use axis_infrastructure::adapters::config::FileConfigProvider;
use axis_infrastructure::adapters::continuity::ContinuityService;
use axis_infrastructure::adapters::dnd::ConfigDndProvider;
use axis_infrastructure::adapters::google_auth::GoogleCloudAuthProvider;
use axis_infrastructure::adapters::idle_inhibit::ConfigIdleInhibitProvider;
use axis_infrastructure::adapters::launcher::CompositeLauncherProvider;
use axis_infrastructure::adapters::lock::{LockGtkHandle, SessionLockProvider};
use axis_infrastructure::adapters::network::NetworkManagerProvider;
use axis_infrastructure::adapters::nightlight::ConfigNightlightProvider;
use axis_infrastructure::adapters::notifications::ZbusNotificationProvider;
use axis_infrastructure::adapters::popups::LocalPopupProvider;
use axis_infrastructure::adapters::power::LogindPowerProvider;
use axis_infrastructure::adapters::pulse::PulseAudioProvider;
use axis_infrastructure::adapters::tray::StatusNotifierTrayProvider;
use axis_infrastructure::adapters::workspaces::NiriWorkspaceProvider;
use std::path::PathBuf;
use std::sync::Arc;

pub struct Providers {
    pub power: Arc<dyn PowerProvider>,
    pub audio: Arc<dyn AudioProvider>,
    pub workspace: Arc<dyn WorkspaceProvider>,
    pub brightness: Arc<dyn BrightnessProvider>,
    pub network: Arc<dyn NetworkProvider>,
    pub bluetooth: Arc<dyn BluetoothProvider>,
    pub config: Arc<dyn ConfigProvider>,
    pub nightlight: Arc<dyn NightlightProvider>,
    pub airplane: Arc<dyn AirplaneProvider>,
    pub appearance: Arc<dyn AppearanceProvider>,
    pub dnd: Arc<dyn DndProvider>,
    pub idle_inhibit: Arc<dyn IdleInhibitProvider>,
    pub clock: Arc<dyn ClockProvider>,
    pub popup: Arc<dyn PopupProvider>,
    pub notification: Arc<dyn NotificationProvider>,
    pub tray: Arc<dyn TrayProvider>,
    pub continuity: Arc<dyn ContinuityProvider>,
    pub continuity_sharing: Arc<dyn ContinuitySharingProvider>,
    pub mpris: Arc<dyn MprisProvider>,
    pub lock: Arc<dyn LockProvider>,
    pub launcher: Arc<CompositeLauncherProvider>,
    pub google_auth: Arc<GoogleCloudAuthProvider>,
    pub google_agenda: Arc<axis_infrastructure::adapters::google_agenda::GoogleAgendaProvider>,
    pub continuity_service: Arc<ContinuityService>,
    pub lock_gtk_handle: LockGtkHandle,
    pub mpris_dbus: Option<Arc<axis_infrastructure::adapters::mpris::MprisDBusProvider>>,
}

pub fn setup(cli: &Cli, rt: &tokio::runtime::Runtime) -> Providers {
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
    let cli_config = AxisConfig {
        appearance: axis_domain::models::config::AppearanceConfig {
            wallpaper: cli.wallpaper.clone(),
            accent_color: cli.accent.as_deref().map(parse_accent).unwrap_or_default(),
            color_scheme: cli
                .mode
                .as_deref()
                .and_then(parse_color_scheme)
                .unwrap_or_default(),
            font: cli.font.clone(),
        },
        ..AxisConfig::default()
    };
    let config_provider = FileConfigProvider::new(cli_config);
    let nightlight_provider: Arc<dyn NightlightProvider> =
        rt.block_on(ConfigNightlightProvider::new(config_provider.clone()));
    let airplane_provider: Arc<dyn AirplaneProvider> =
        rt.block_on(ConfigAirplaneProvider::new(config_provider.clone()));
    let appearance_provider: Arc<dyn AppearanceProvider> =
        rt.block_on(ConfigAppearanceProvider::new(config_provider.clone()));
    let dnd_provider: Arc<dyn DndProvider> =
        rt.block_on(ConfigDndProvider::new(config_provider.clone()));
    let idle_inhibit_provider: Arc<dyn IdleInhibitProvider> =
        rt.block_on(ConfigIdleInhibitProvider::new(config_provider.clone()));
    let clock_provider: Arc<dyn ClockProvider> = SystemClockProvider::new();
    let popup_provider: Arc<dyn PopupProvider> = LocalPopupProvider::new();
    let launcher_provider = CompositeLauncherProvider::new();
    let config_dir = dirs::config_dir()
        .unwrap_or(PathBuf::from("."))
        .join("axis");
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
    let continuity_sharing_provider: Arc<dyn ContinuitySharingProvider> =
        continuity_service.clone();
    let (mpris_provider, mpris_dbus_provider): (
        Arc<dyn MprisProvider>,
        Option<Arc<axis_infrastructure::adapters::mpris::MprisDBusProvider>>,
    ) = rt.block_on(async {
        match axis_infrastructure::adapters::mpris::MprisDBusProvider::new().await {
            Ok(p) => {
                let dbus = p.clone();
                (p as Arc<dyn MprisProvider>, Some(dbus))
            }
            Err(_) => {
                log::warn!("[mpris] MPRIS not available, using mock");
                let mock: Arc<dyn MprisProvider> =
                    axis_infrastructure::mocks::mpris::MockMprisProvider::new();
                (mock, None)
            }
        }
    });
    let initial_config = config_provider.get().unwrap_or_else(|e| {
        log::warn!("[providers] config get failed: {e}");
        AxisConfig::default()
    });
    let (lock_provider_arc, lock_gtk_handle) = SessionLockProvider::new(
        idle_inhibit_provider.clone(),
        config_provider.clone(),
        initial_config.idle.lock_timeout_seconds,
        initial_config.idle.blank_timeout_seconds,
        initial_config.idle.sleep_timeout_seconds,
    );
    let lock_provider: Arc<dyn LockProvider> = lock_provider_arc;
    let google_calendar =
        axis_infrastructure::adapters::google_calendar::GoogleCalendarProvider::new(
            google_auth.clone(),
        );
    let google_tasks =
        axis_infrastructure::adapters::google_tasks::GoogleTasksProvider::new(google_auth.clone());
    let google_agenda = axis_infrastructure::adapters::google_agenda::GoogleAgendaProvider::new(
        google_calendar,
        google_tasks,
    );
    Providers {
        power: power_provider,
        audio: audio_provider,
        workspace: workspace_provider,
        brightness: brightness_provider,
        network: network_provider,
        bluetooth: bluetooth_provider,
        config: config_provider,
        nightlight: nightlight_provider,
        airplane: airplane_provider,
        appearance: appearance_provider,
        dnd: dnd_provider,
        idle_inhibit: idle_inhibit_provider,
        clock: clock_provider,
        popup: popup_provider,
        notification: notification_provider,
        tray: tray_provider,
        continuity: continuity_provider,
        continuity_sharing: continuity_sharing_provider,
        mpris: mpris_provider,
        lock: lock_provider,
        launcher: launcher_provider,
        google_auth,
        google_agenda,
        continuity_service,
        lock_gtk_handle,
        mpris_dbus: mpris_dbus_provider,
    }
}
