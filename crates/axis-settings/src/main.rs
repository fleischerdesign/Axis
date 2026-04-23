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
use widgets::accounts_page::AccountsPage;
use widgets::appearance_page::AppearancePage;
use widgets::sidebar::Sidebar;
use widgets::window::SettingsWindow;

use axis_application::use_cases::cloud::subscribe::SubscribeToCloudUpdatesUseCase;
use axis_application::use_cases::cloud::authenticate::AuthenticateAccountUseCase;
use axis_application::use_cases::appearance::subscribe::SubscribeToAppearanceUseCase;
use axis_application::use_cases::appearance::set_accent::SetAccentColorUseCase;
use axis_application::use_cases::appearance::set_scheme::SetColorSchemeUseCase;
use axis_application::use_cases::appearance::set_wallpaper::SetWallpaperUseCase;

use axis_infrastructure::adapters::cloud::LocalCloudProvider;
use axis_infrastructure::adapters::google_auth::GoogleCloudAdapter;
use axis_infrastructure::adapters::appearance::ConfigAppearanceProvider;
use axis_infrastructure::adapters::config::FileConfigProvider;
use axis_domain::models::config::AxisConfig;
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
        build_ui(app, theme_provider.get().expect("theme provider not initialized").clone());
    });
    
    app.run()
}

fn build_ui(app: &adw::Application, theme_css: Rc<gtk4::CssProvider>) {
    let config_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("axis");
    let _ = std::fs::create_dir_all(&config_dir);

    // 1. Infrastructure
    let config_provider = FileConfigProvider::new(AxisConfig::default());
    let cloud_provider = Arc::new(LocalCloudProvider::new(config_dir.clone()));
    let google_auth = Arc::new(GoogleCloudAdapter::new(config_dir.clone()));
    let appearance_provider = ConfigAppearanceProvider::new(config_provider.clone());

    // 2. Use Cases
    let subscribe_cloud = Arc::new(SubscribeToCloudUpdatesUseCase::new(cloud_provider.clone()));
    let authenticate_cloud = Arc::new(AuthenticateAccountUseCase::new(google_auth.clone(), cloud_provider.clone()));
    
    let subscribe_appearance = Arc::new(SubscribeToAppearanceUseCase::new(appearance_provider.clone()));
    let set_accent = Arc::new(SetAccentColorUseCase::new(appearance_provider.clone()));
    let set_scheme = Arc::new(SetColorSchemeUseCase::new(appearance_provider.clone()));
    let set_wallpaper = Arc::new(SetWallpaperUseCase::new(appearance_provider.clone()));

    // 3. Presenters
    let accounts_presenter = Rc::new(AccountsPresenter::new(
        subscribe_cloud,
        authenticate_cloud,
    ));

    let appearance_presenter = Rc::new(AppearancePresenter::new(
        subscribe_appearance,
        set_accent,
        set_scheme,
        set_wallpaper,
    ));

    let initial_pages = vec![
        PageDescriptor {
            id: "appearance".to_string(),
            title: "Appearance".to_string(),
            icon: "preferences-desktop-wallpaper-symbolic".to_string(),
        },
        PageDescriptor {
            id: "accounts".to_string(),
            title: "Accounts".to_string(),
            icon: "avatar-default-symbolic".to_string(),
        },
    ];
    let navigation_presenter = Rc::new(NavigationPresenter::new(initial_pages));

    // 4. Widgets
    let accounts_page = AccountsPage::new(accounts_presenter.clone());
    let appearance_page = AppearancePage::new(appearance_presenter.clone());
    let sidebar = Sidebar::new(navigation_presenter.clone());
    let settings_window = SettingsWindow::new(app, sidebar.widget().upcast_ref());

    // 5. Services
    let theme_service = ThemeService::new(theme_css);
    appearance_presenter.add_view(Box::new(theme_service));

    // Register pages in the window
    settings_window.register_page_widget("appearance", "Appearance", appearance_page.widget());
    settings_window.register_page_widget("accounts", "Accounts", accounts_page.widget());
    
    // 6. Wiring (Reactive bindings)
    let ap_run = accounts_presenter.clone();
    glib::spawn_future_local(async move {
        ap_run.run().await;
    });

    let app_run = appearance_presenter.clone();
    let app_page_c = appearance_page.clone();
    glib::spawn_future_local(async move {
        app_run.bind(Box::new(app_page_c)).await;
        app_run.run().await; // Dieser Aufruf fehlte!
    });

    let nav_run = navigation_presenter.clone();
    glib::spawn_future_local(async move {
        nav_run.run().await;
    });

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
