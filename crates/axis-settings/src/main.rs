use libadwaita::prelude::*;
use libadwaita as adw;
use gtk4::{glib, gdk};
use chrono::Local;
use std::rc::Rc;
use std::sync::Arc;
use std::path::PathBuf;

mod presentation;
mod widgets;

use presentation::accounts::AccountsPresenter;
use presentation::appearance::AppearancePresenter;
use presentation::navigation::{NavigationPresenter, PageDescriptor};
use presentation::network::NetworkPresenter;
use presentation::bluetooth::BluetoothPresenter;

use widgets::accounts_page::AccountsPage;
use widgets::appearance_page::AppearancePage;
use widgets::network_page::NetworkPage;
use widgets::bluetooth_page::BluetoothPage;
use widgets::sidebar::Sidebar;
use widgets::window::SettingsWindow;

use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::cloud::authenticate::AuthenticateAccountUseCase;
use axis_application::use_cases::appearance::set_accent::SetAccentColorUseCase;
use axis_application::use_cases::appearance::set_scheme::SetColorSchemeUseCase;
use axis_application::use_cases::appearance::set_wallpaper::SetWallpaperUseCase;

use axis_application::use_cases::network::scan_wifi::ScanWifiUseCase;
use axis_application::use_cases::network::connect_to_ap::ConnectToApUseCase;
use axis_application::use_cases::network::disconnect_wifi::DisconnectWifiUseCase;

use axis_application::use_cases::bluetooth::connect::ConnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::disconnect::DisconnectBluetoothDeviceUseCase;
use axis_application::use_cases::bluetooth::set_powered::SetBluetoothPoweredUseCase;
use axis_application::use_cases::bluetooth::start_scan::StartBluetoothScanUseCase;
use axis_application::use_cases::bluetooth::stop_scan::StopBluetoothScanUseCase;

use axis_infrastructure::adapters::cloud::LocalCloudProvider;
use axis_infrastructure::adapters::google_auth::GoogleCloudAdapter;
use axis_infrastructure::adapters::appearance::ConfigAppearanceProvider;
use axis_infrastructure::adapters::config::FileConfigProvider;
use axis_infrastructure::adapters::network::NetworkManagerProvider;
use axis_infrastructure::adapters::bluetooth::BlueZProvider;
use axis_infrastructure::adapters::niri_layout::NiriLayoutProvider;

use axis_domain::models::config::AxisConfig;
use axis_domain::ports::cloud::CloudProvider;
use axis_domain::ports::appearance::AppearanceProvider;
use axis_domain::ports::network::NetworkProvider;
use axis_domain::ports::bluetooth::BluetoothProvider;
use axis_domain::ports::layout::LayoutProvider;
use axis_presentation::ThemeService;

fn main() -> glib::ExitCode {
    setup_logger().expect("Failed to initialize logger");
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    let app = adw::Application::builder()
        .application_id("design.fleischer.axis.settings")
        .build();

    let theme_provider: Rc<std::cell::OnceCell<Rc<gtk4::CssProvider>>> = Rc::new(std::cell::OnceCell::new());
    let theme_provider_c = theme_provider.clone();

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
        let _ = theme_provider_c.set(theme_css);
    });

    app.connect_activate(move |app| {
        let theme_css = theme_provider.get().cloned().unwrap_or_else(|| {
            log::error!("theme provider not initialized, falling back to empty CSS");
            Rc::new(gtk4::CssProvider::new())
        });
        build_ui(app, theme_css, &rt);
    });
    
    app.run()
}

fn build_ui(app: &adw::Application, theme_css: Rc<gtk4::CssProvider>, rt: &tokio::runtime::Runtime) {
    let config_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("axis");
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        log::warn!("[settings] Failed to create config dir: {e}");
    }

    // 1. Infrastructure
    let config_provider = FileConfigProvider::new(AxisConfig::default());
    let cloud_provider: Arc<dyn CloudProvider> = LocalCloudProvider::new(config_dir.clone());
    let google_auth = Arc::new(GoogleCloudAdapter::new(config_dir.clone()));
    let appearance_provider: Arc<dyn AppearanceProvider> = rt.block_on(ConfigAppearanceProvider::new(config_provider.clone()));
    let niri_layout_provider: Arc<dyn LayoutProvider> = NiriLayoutProvider::new(config_dir.clone());
    
    let network_provider: Arc<dyn NetworkProvider> = rt.block_on(async {
        NetworkManagerProvider::new().await.expect("Failed to connect to NetworkManager")
    });
    let bluetooth_provider: Arc<dyn BluetoothProvider> = rt.block_on(async {
        BlueZProvider::new().await.expect("Failed to connect to BlueZ")
    });

    // 2. Use Cases
    let subscribe_cloud = Arc::new(SubscribeUseCase::new(cloud_provider.clone()));
    let authenticate_cloud = Arc::new(AuthenticateAccountUseCase::new(google_auth.clone(), cloud_provider.clone()));
    
    let subscribe_appearance = Arc::new(SubscribeUseCase::new(appearance_provider.clone()));
    let set_accent = Arc::new(SetAccentColorUseCase::new(appearance_provider.clone(), niri_layout_provider));
    let set_scheme = Arc::new(SetColorSchemeUseCase::new(appearance_provider.clone()));
    let set_wallpaper = Arc::new(SetWallpaperUseCase::new(appearance_provider.clone()));

    let subscribe_network = Arc::new(SubscribeUseCase::new(network_provider.clone()));
    let get_network_status = Arc::new(GetStatusUseCase::new(network_provider.clone()));
    let scan_wifi = Arc::new(ScanWifiUseCase::new(network_provider.clone()));
    let connect_to_ap = Arc::new(ConnectToApUseCase::new(network_provider.clone()));
    let disconnect_wifi = Arc::new(DisconnectWifiUseCase::new(network_provider.clone()));

    let subscribe_bluetooth = Arc::new(SubscribeUseCase::new(bluetooth_provider.clone()));
    let get_bluetooth_status = Arc::new(GetStatusUseCase::new(bluetooth_provider.clone()));
    let bt_connect = Arc::new(ConnectBluetoothDeviceUseCase::new(bluetooth_provider.clone()));
    let bt_disconnect = Arc::new(DisconnectBluetoothDeviceUseCase::new(bluetooth_provider.clone()));
    let bt_set_powered = Arc::new(SetBluetoothPoweredUseCase::new(bluetooth_provider.clone()));
    let bt_start_scan = Arc::new(StartBluetoothScanUseCase::new(bluetooth_provider.clone()));
    let bt_stop_scan = Arc::new(StopBluetoothScanUseCase::new(bluetooth_provider.clone()));

    // 3. Presenters
    let accounts_presenter = Rc::new(AccountsPresenter::new(subscribe_cloud, authenticate_cloud));
    let appearance_presenter = Rc::new(AppearancePresenter::new(subscribe_appearance, set_accent, set_scheme, set_wallpaper));
    let network_presenter = Rc::new(NetworkPresenter::new(subscribe_network, get_network_status, scan_wifi, connect_to_ap, disconnect_wifi, rt));
    let bluetooth_presenter = Rc::new(BluetoothPresenter::new(subscribe_bluetooth, get_bluetooth_status, bt_connect, bt_disconnect, bt_set_powered, bt_start_scan, bt_stop_scan, rt));

    let initial_pages = vec![
        PageDescriptor { id: "appearance".to_string(), title: "Appearance".to_string(), icon: "preferences-desktop-wallpaper-symbolic".to_string() },
        PageDescriptor { id: "network".to_string(), title: "Network".to_string(), icon: "network-wireless-symbolic".to_string() },
        PageDescriptor { id: "bluetooth".to_string(), title: "Bluetooth".to_string(), icon: "bluetooth-active-symbolic".to_string() },
        PageDescriptor { id: "accounts".to_string(), title: "Accounts".to_string(), icon: "avatar-default-symbolic".to_string() },
    ];
    let navigation_presenter = Rc::new(NavigationPresenter::new(initial_pages));

    // 4. Widgets
    let accounts_page = AccountsPage::new(accounts_presenter.clone());
    let appearance_page = AppearancePage::new(appearance_presenter.clone());
    let network_page = NetworkPage::new(network_presenter.clone());
    let bluetooth_page = BluetoothPage::new(bluetooth_presenter.clone());
    let sidebar = Sidebar::new(navigation_presenter.clone());
    let settings_window = SettingsWindow::new(app, sidebar.widget().upcast_ref());

    // 5. Services
    let theme_service = ThemeService::new(theme_css);
    appearance_presenter.add_view(Box::new(theme_service));

    // Register pages
    settings_window.register_page_widget("appearance", "Appearance", appearance_page.widget());
    settings_window.register_page_widget("network", "Network", network_page.widget());
    settings_window.register_page_widget("bluetooth", "Bluetooth", bluetooth_page.widget());
    settings_window.register_page_widget("accounts", "Accounts", accounts_page.widget());
    
    // 6. Wiring (Reactive bindings)
    let ap_run = accounts_presenter.clone();
    glib::spawn_future_local(async move { ap_run.run().await; });

    let app_run = appearance_presenter.clone();
    let app_page_c = appearance_page.clone();
    glib::spawn_future_local(async move {
        app_run.bind(Box::new(app_page_c)).await;
        app_run.run().await;
    });

    let net_run = network_presenter.clone();
    let net_page_c = network_page.clone();
    glib::spawn_future_local(async move {
        net_run.bind(Box::new(net_page_c)).await;
        net_run.run_sync().await;
    });

    let bt_run = bluetooth_presenter.clone();
    let bt_page_c = bluetooth_page.clone();
    glib::spawn_future_local(async move {
        bt_run.bind(Box::new(bt_page_c)).await;
        bt_run.run_sync().await;
    });

    let nav_run = navigation_presenter.clone();
    glib::spawn_future_local(async move { nav_run.run().await; });

    accounts_presenter.add_view(Box::new(accounts_page));
    navigation_presenter.add_view(Box::new(sidebar));
    navigation_presenter.add_view(Box::new(settings_window.clone()));

    settings_window.present();
}

fn setup_logger() -> Result<(), fern::InitError> {
    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
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
