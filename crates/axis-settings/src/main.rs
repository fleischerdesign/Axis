use libadwaita::prelude::*;
use libadwaita as adw;
use gtk4::{glib, gdk};
use std::rc::Rc;
use std::sync::Arc;
use std::path::PathBuf;

mod presentation;
mod widgets;

use presentation::accounts::AccountsPresenter;
use presentation::navigation::{NavigationPresenter, PageDescriptor};
use widgets::accounts_page::AccountsPage;
use widgets::sidebar::Sidebar;
use widgets::window::SettingsWindow;

use axis_application::use_cases::cloud::subscribe::SubscribeToCloudUpdatesUseCase;
use axis_application::use_cases::cloud::authenticate::AuthenticateAccountUseCase;
use axis_infrastructure::adapters::cloud::LocalCloudProvider;
use axis_infrastructure::adapters::google_auth::GoogleCloudAdapter;

fn main() -> glib::ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    let app = adw::Application::builder()
        .application_id("design.fleischer.axis.settings")
        .build();

    app.connect_activate(move |app| {
        setup_css();
        build_ui(app);
    });
    
    app.run()
}

fn setup_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_ui(app: &adw::Application) {
    let config_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("axis");
    let _ = std::fs::create_dir_all(&config_dir);

    // 1. Infrastructure
    let cloud_provider = Arc::new(LocalCloudProvider::new(config_dir.clone()));
    let google_auth = Arc::new(GoogleCloudAdapter::new(config_dir.clone()));

    // 2. Use Cases
    let subscribe_cloud = Arc::new(SubscribeToCloudUpdatesUseCase::new(cloud_provider.clone()));
    let authenticate_cloud = Arc::new(AuthenticateAccountUseCase::new(google_auth.clone()));

    // 3. Presenters
    let accounts_presenter = Rc::new(AccountsPresenter::new(
        subscribe_cloud,
        authenticate_cloud,
    ));

    let initial_pages = vec![
        PageDescriptor {
            id: "accounts".to_string(),
            title: "Accounts".to_string(),
            icon: "avatar-default-symbolic".to_string(),
        },
        PageDescriptor {
            id: "appearance".to_string(),
            title: "Appearance".to_string(),
            icon: "preferences-desktop-wallpaper-symbolic".to_string(),
        },
    ];
    let navigation_presenter = Rc::new(NavigationPresenter::new(initial_pages));

    // 4. Widgets
    let accounts_page = AccountsPage::new(accounts_presenter.clone());
    let sidebar = Sidebar::new(navigation_presenter.clone());
    let settings_window = SettingsWindow::new(app, sidebar.widget().upcast_ref());

    // Register pages in the window
    settings_window.register_page_widget("accounts", "Accounts", accounts_page.widget());
    
    let appearance_placeholder = adw::StatusPage::builder()
        .title("Appearance")
        .description("Under migration...")
        .build();
    settings_window.register_page_widget("appearance", "Appearance", &appearance_placeholder);

    // 5. Wiring (Reactive bindings)
    let ap_run = accounts_presenter.clone();
    glib::spawn_future_local(async move {
        ap_run.run().await;
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
