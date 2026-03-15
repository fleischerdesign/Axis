mod services;
mod widgets;

use gtk4::prelude::*;
use libadwaita::prelude::*;
use crate::widgets::{Bar, QuickSettingsPopup, WorkspacePopup};

#[tokio::main]
async fn main() {
    let application = libadwaita::Application::builder()
        .application_id("com.github.carp.shell")
        .build();

    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(app: &libadwaita::Application) {
    // CSS laden
    let provider = gtk4::CssProvider::new();
    provider.load_from_path("src/style.css");
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::PreferDark);

    // --- KOMPONENTEN INITIALISIEREN ---
    let bar = Bar::new(app);
    let ws_popup = WorkspacePopup::new(app, &bar.clock_label, &bar.ws_label);
    let qs_popup = QuickSettingsPopup::new(app, &bar.vol_icon);

    // --- INTERACTION ---
    let ws_click = gtk4::GestureClick::new();
    ws_click.connect_pressed(move |_, _, _, _| { ws_popup.toggle(); });
    bar.center_island.add_controller(ws_click);

    let qs_click = gtk4::GestureClick::new();
    qs_click.connect_pressed(move |_, _, _, _| { qs_popup.toggle(); });
    bar.status_island.add_controller(qs_click);

    bar.window.present();
}
