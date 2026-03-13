use gtk4::prelude::*;
use libadwaita::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use chrono::{Local};

fn main() {
    let application = libadwaita::Application::builder()
        .application_id("org.carp.shell")
        .build();

    application.connect_activate(|app| {
        // Dark Mode erzwingen
        let style_manager = libadwaita::StyleManager::default();
        style_manager.set_color_scheme(libadwaita::ColorScheme::PreferDark);

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Carp Bottom Bar")
            .build();

        // Layer Shell Setup
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        
        // Diese Anchors zwingen das FENSTER auf volle Breite
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        
        // 54 Pixel Platz reservieren (Bar ist 44 hoch + 10 margin)
        window.set_exclusive_zone(54); 

        // Root Container
        let root = gtk4::CenterBox::new();
        root.set_hexpand(true); // CRITICAL: Sorgt dafür, dass die Box das Fenster ausfüllt
        root.set_margin_bottom(10);
        root.set_margin_start(12);
        root.set_margin_end(12);
        root.set_height_request(44);
        root.set_valign(gtk4::Align::Center);

        // --- 1. Launcher Island ---
        let launcher_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        launcher_box.add_css_class("island");
        launcher_box.append(&gtk4::Image::from_icon_name("view-app-grid-symbolic"));
        root.set_start_widget(Some(&launcher_box));

        // --- 2. Center Island (Clock) ---
        let center_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        center_box.add_css_class("island");
        
        let clock_label = gtk4::Label::new(None);
        clock_label.add_css_class("clock-label");
        center_box.append(&clock_label);
        
        let clock_label_clone = clock_label.clone();
        gtk4::glib::timeout_add_seconds_local(1, move || {
            let now = Local::now();
            clock_label_clone.set_text(&now.format("%a, %d. %b  %H:%M").to_string());
            gtk4::glib::ControlFlow::Continue
        });
        root.set_center_widget(Some(&center_box));

        // --- 3. Status Island ---
        let status_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        status_box.add_css_class("island");
        status_box.append(&gtk4::Image::from_icon_name("network-wireless-signal-excellent-symbolic"));
        status_box.append(&gtk4::Image::from_icon_name("audio-volume-high-symbolic"));
        status_box.append(&gtk4::Image::from_icon_name("battery-full-symbolic"));
        root.set_end_widget(Some(&status_box));

        window.set_child(Some(&root));
        
        // CSS Styling
        let provider = gtk4::CssProvider::new();
        provider.load_from_data("
            window {
                background: transparent;
            }
            .island {
                background-color: #242424; 
                background-color: @card_bg_color;
                color: white;
                color: @card_fg_color;
                border: 1px solid rgba(255, 255, 255, 0.08);
                border-radius: 14px;
                padding: 4px 16px;
                margin: 0;
                box-shadow: 0 2px 8px rgba(0,0,0,0.4);
                min-height: 32px;
            }
            .clock-label {
                font-weight: 600;
                font-size: 13px;
            }
        ");
        
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        window.present();
    });

    application.run();
}
