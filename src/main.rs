mod services;
mod widgets;

use chrono::Local;
use futures_util::StreamExt;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
// use libadwaita::prelude::*; // Entfernt, da ungenutzt
use std::cell::RefCell;
use std::rc::Rc;

use services::audio::AudioService;
use services::niri::NiriService;
use services::network::{NetworkService, NetworkCmd};
use services::bluetooth::BluetoothService;
use widgets::island::Island;

#[tokio::main]
async fn main() {
    let application = libadwaita::Application::builder()
        .application_id("org.carp.shell")
        .build();

    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(app: &libadwaita::Application) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_path("src/style.css");
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let style_manager = libadwaita::StyleManager::default();
    style_manager.set_color_scheme(libadwaita::ColorScheme::PreferDark);

    // --- STATE ---
    let is_ws_open = Rc::new(RefCell::new(false));
    let is_qs_open = Rc::new(RefCell::new(false));
    let is_wifi_on = Rc::new(RefCell::new(false));
    let is_bt_on = Rc::new(RefCell::new(false));
    let is_net_on = Rc::new(RefCell::new(false));

    // --- WINDOWS ---
    let bar_window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Carp Bottom Bar")
        .build();
    bar_window.init_layer_shell();
    bar_window.set_layer(Layer::Top);
    bar_window.set_anchor(Edge::Bottom, true);
    bar_window.set_anchor(Edge::Left, true);
    bar_window.set_anchor(Edge::Right, true);
    bar_window.set_exclusive_zone(54);

    let ws_popup = gtk4::Window::builder()
        .application(app)
        .title("Carp Workspace Popup")
        .visible(false)
        .build();
    ws_popup.init_layer_shell();
    ws_popup.set_layer(Layer::Overlay);
    ws_popup.set_anchor(Edge::Bottom, true);
    ws_popup.set_margin(Edge::Bottom, 10);

    let ws_revealer = gtk4::Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::Crossfade)
        .transition_duration(250)
        .build();
    let shelf_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
    shelf_box.add_css_class("workspace-shelf");
    ws_revealer.set_child(Some(&shelf_box));
    ws_popup.set_child(Some(&ws_revealer));

    // --- QUICK SETTINGS POPUP ---
    let qs_popup = gtk4::Window::builder()
        .application(app)
        .title("Carp Quick Settings")
        .visible(false)
        .build();
    qs_popup.init_layer_shell();
    qs_popup.set_layer(Layer::Overlay);
    qs_popup.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
    qs_popup.set_anchor(Edge::Bottom, true);
    qs_popup.set_anchor(Edge::Right, true);
    qs_popup.set_margin(Edge::Bottom, 10);
    qs_popup.set_margin(Edge::Right, 10);

    let qs_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    qs_container.add_css_class("qs-panel");
    qs_container.set_width_request(340);

    let qs_stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .transition_duration(250)
        .vhomogeneous(false)
        .hhomogeneous(false)
        .interpolate_size(true)
        .build();

    // --- PAGE 1: MAIN GRID ---
    let main_page = gtk4::Box::new(gtk4::Orientation::Vertical, 20);
    
    // 1. Grid of Toggles
    let grid = gtk4::Grid::new();
    grid.set_column_spacing(12);
    grid.set_row_spacing(12);
    grid.set_column_homogeneous(true);

    let (wifi_tile, wifi_toggle, wifi_arrow) = create_tile(
        "Wi-Fi",
        "network-wireless-signal-excellent-symbolic",
        true,
        true,
    );
    let (eth_tile, eth_toggle, _eth_arrow) = create_tile(
        "Ethernet",
        "network-wired-symbolic",
        false,
        false,
    );
    let (bt_tile, bt_toggle, _bt_arrow) = create_tile("Bluetooth", "bluetooth-active-symbolic", false, true);
    let (night_tile, _, _) = create_tile("Night Light", "night-light-symbolic", false, false);
    let (airplane_tile, _, _) = create_tile("Airplane", "airplane-mode-symbolic", false, false);

    grid.attach(&wifi_tile, 0, 0, 1, 1);
    grid.attach(&eth_tile, 1, 0, 1, 1);
    grid.attach(&bt_tile, 0, 1, 1, 1);
    grid.attach(&night_tile, 1, 1, 1, 1);
    grid.attach(&airplane_tile, 0, 2, 1, 1);

    // 2. Volume Slider (Pill Style)
    let slider_overlay = gtk4::Overlay::new();
    slider_overlay.add_css_class("volume-slider");
    slider_overlay.set_hexpand(true);
    let vol_slider = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 1.0, 0.01);
    vol_slider.set_hexpand(true);
    let popup_vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
    popup_vol_icon.set_pixel_size(22);
    popup_vol_icon.set_margin_start(22);
    popup_vol_icon.set_halign(gtk4::Align::Start);
    popup_vol_icon.set_valign(gtk4::Align::Center);
    popup_vol_icon.set_can_target(false);
    slider_overlay.set_child(Some(&vol_slider));
    slider_overlay.add_overlay(&popup_vol_icon);

    // 3. Bottom Row (Actions)
    let bottom_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

    let battery_content = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let battery_icon = gtk4::Image::from_icon_name("battery-full-symbolic");
    let battery_label = gtk4::Label::new(Some("85%"));

    battery_content.append(&battery_icon);
    battery_content.append(&battery_label);

    let battery_btn = gtk4::Button::builder()
        .child(&battery_content)
        .css_classes(vec!["qs-battery-btn".to_string()])
        .build();

    let power_btn = create_bottom_btn("system-shutdown-symbolic");
    let lock_btn = create_bottom_btn("system-lock-screen-symbolic");
    let settings_btn = create_bottom_btn("emblem-system-symbolic");
    let screenshot_btn = create_bottom_btn("camera-photo-symbolic");

    bottom_row.append(&battery_btn);
    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    bottom_row.append(&spacer);
    bottom_row.append(&screenshot_btn);
    bottom_row.append(&settings_btn);
    bottom_row.append(&lock_btn);
    bottom_row.append(&power_btn);

    main_page.append(&grid);
    main_page.append(&slider_overlay);
    main_page.append(&bottom_row);

    // --- PAGE 2: WIFI SUB-PAGE ---
    let wifi_page = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    let wifi_header = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let back_btn = gtk4::Button::builder()
        .icon_name("go-previous-symbolic")
        .css_classes(vec!["qs-back-btn".to_string()])
        .build();
    let wifi_title = gtk4::Label::builder()
        .label("Wi-Fi")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["qs-subpage-title".to_string()])
        .build();
    wifi_header.append(&back_btn);
    wifi_header.append(&wifi_title);

    let wifi_list = gtk4::ListBox::builder()
        .css_classes(vec!["qs-list".to_string()])
        .selection_mode(gtk4::SelectionMode::None) // Liste verwaltet nur Fokus, keine Selektion
        .build();

    wifi_page.append(&wifi_header);
    wifi_page.append(&wifi_list);

    // Add pages to stack
    qs_stack.add_named(&main_page, Some("main"));
    qs_stack.add_named(&wifi_page, Some("wifi"));

    let stack_back_clone = qs_stack.clone();
    back_btn.connect_clicked(move |_| {
        stack_back_clone.set_visible_child_name("main");
    });

    qs_container.append(&qs_stack);

    let qs_revealer = gtk4::Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::Crossfade)
        .transition_duration(250)
        .build();
    qs_revealer.set_child(Some(&qs_container));
    qs_popup.set_child(Some(&qs_revealer));

    // --- BAR UI ---
    let root = gtk4::CenterBox::new();
    root.set_margin_bottom(10);
    root.set_height_request(44);

    let launcher_island = Island::new(0);
    launcher_island.append(&gtk4::Image::from_icon_name("view-app-grid-symbolic"));
    root.set_start_widget(Some(&launcher_island.container));

    let center_island = Island::new(12);
    center_island
        .container
        .set_cursor_from_name(Some("pointer"));
    let ws_label = gtk4::Label::new(None);
    ws_label.add_css_class("workspace-label");
    let clock_label = gtk4::Label::new(None);
    clock_label.add_css_class("clock-label");
    center_island.append(&ws_label);
    center_island.append(&gtk4::Separator::new(gtk4::Orientation::Vertical));
    center_island.append(&clock_label);
    root.set_center_widget(Some(&center_island.container));

    let status_island = Island::new(12);
    status_island
        .container
        .set_cursor_from_name(Some("pointer"));
    status_island.append(&gtk4::Image::from_icon_name(
        "network-wireless-signal-excellent-symbolic",
    ));
    let vol_icon = gtk4::Image::from_icon_name("audio-volume-high-symbolic");
    status_island.append(&vol_icon);
    status_island.append(&gtk4::Image::from_icon_name("battery-full-symbolic"));
    root.set_end_widget(Some(&status_island.container));

    bar_window.set_child(Some(&root));

    // --- INTERACTION ---
    let ws_click = gtk4::GestureClick::new();
    let ws_popup_clone = ws_popup.clone();
    let ws_revealer_clone = ws_revealer.clone();
    let ws_open_clone = is_ws_open.clone();
    ws_click.connect_pressed(move |_, _, _, _| {
        let mut open = ws_open_clone.borrow_mut();
        *open = !*open;
        if *open {
            ws_popup_clone.set_visible(true);
            ws_revealer_clone.set_reveal_child(true);
        } else {
            ws_revealer_clone.set_reveal_child(false);
            let win = ws_popup_clone.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(280), move || {
                win.set_visible(false);
                gtk4::glib::ControlFlow::Break
            });
        }
    });
    center_island.container.add_controller(ws_click);

    let qs_click = gtk4::GestureClick::new();
    let qs_popup_clone = qs_popup.clone();
    let qs_revealer_clone = qs_revealer.clone();
    let qs_open_clone = is_qs_open.clone();
    qs_click.connect_pressed(move |_, _, _, _| {
        let mut open = qs_open_clone.borrow_mut();
        *open = !*open;
        if *open {
            qs_popup_clone.set_visible(true);
            qs_revealer_clone.set_reveal_child(true);
        } else {
            qs_revealer_clone.set_reveal_child(false);
            let win = qs_popup_clone.clone();
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(280), move || {
                win.set_visible(false);
                gtk4::glib::ControlFlow::Break
            });
        }
    });
    status_island.container.add_controller(qs_click);

    // --- DATA SERVICES ---
    let mut niri_rx = NiriService::spawn();
    let (mut audio_rx, audio_tx) = AudioService::spawn();

    // Niri & Clock Loop
    let ws_label_c = ws_label.clone();
    let clock_label_c = clock_label.clone();
    let shelf_box_c = shelf_box.clone();
    let ws_popup_c = ws_popup.clone();
    gtk4::glib::MainContext::default().spawn_local(async move {
        while let Some(data) = niri_rx.next().await {
            let mut workspaces = data.workspaces;
            workspaces.sort_by_key(|w| w.id);
            let mut windows = data.windows;
            windows.sort_by_key(|w| w.layout.pos_in_scrolling_layout.unwrap_or((0, 0)));
            clock_label_c.set_text(&Local::now().format("%H:%M").to_string());
            let mut ws_markup = String::new();
            for ws in &workspaces {
                if ws.is_active {
                    ws_markup.push_str(&format!(" <b>{}</b> ", ws.id));
                } else {
                    ws_markup.push_str(&format!(" {} ", ws.id));
                }
            }
            ws_label_c.set_markup(&ws_markup);
            while let Some(child) = shelf_box_c.first_child() {
                shelf_box_c.remove(&child);
            }
            for ws in workspaces {
                let (m_w, m_h) =
                    if let Some(o) = data.outputs.get(ws.output.as_deref().unwrap_or("")) {
                        if let Some(l) = &o.logical {
                            (l.width as f64, l.height as f64)
                        } else {
                            (1920.0, 1080.0)
                        }
                    } else {
                        (1920.0, 1080.0)
                    };
                let cw = 220.0;
                let m = 15.0;
                let ch = ((cw - m * 2.0) / (m_w / m_h)) + m * 2.0;
                let card = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Vertical)
                    .width_request(cw as i32)
                    .css_classes(vec!["workspace-card".to_string()])
                    .build();
                if ws.is_active {
                    card.add_css_class("active");
                }
                let sc = gtk4::ScrolledWindow::builder()
                    .width_request(cw as i32)
                    .height_request(ch as i32)
                    .hscrollbar_policy(gtk4::PolicyType::Never)
                    .vscrollbar_policy(gtk4::PolicyType::Never)
                    .css_classes(vec!["workspace-preview".to_string()])
                    .build();
                let st = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                st.set_halign(gtk4::Align::Center);
                st.set_valign(gtk4::Align::Center);
                st.set_margin_start(m as i32);
                st.set_margin_end(m as i32);
                st.set_margin_top(m as i32);
                st.set_margin_bottom(m as i32);
                sc.set_child(Some(&st));
                let scale = (cw - m * 2.0) / m_w;
                let mut cur_col: Option<gtk4::Box> = None;
                let mut last_idx = None;
                for win in &windows {
                    if win.workspace_id == Some(ws.id) {
                        let (w_r, h_r) = win.layout.tile_size;
                        let c_idx = win.layout.pos_in_scrolling_layout.map(|p| p.0).unwrap_or(0);
                        if Some(c_idx) != last_idx || cur_col.is_none() {
                            let nc = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
                            nc.set_valign(gtk4::Align::Center);
                            st.append(&nc);
                            cur_col = Some(nc);
                            last_idx = Some(c_idx);
                        }
                        if let Some(cb) = &cur_col {
                            let wb = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                            wb.set_size_request((w_r * scale) as i32, (h_r * scale) as i32);
                            wb.add_css_class("preview-window");
                            if win.is_focused {
                                wb.add_css_class("focused");
                            }
                            let ic = gtk4::Image::from_icon_name(
                                win.app_id.as_deref().unwrap_or("application-x-executable"),
                            );
                            ic.set_pixel_size(((h_r * scale) / 2.0) as i32);
                            ic.set_halign(gtk4::Align::Center);
                            ic.set_valign(gtk4::Align::Center);
                            ic.set_hexpand(true);
                            ic.set_vexpand(true);
                            wb.append(&ic);
                            cb.append(&wb);
                        }
                    }
                }
                card.append(&sc);
                card.append(&gtk4::Label::new(Some(&format!("Workspace {}", ws.id))));
                shelf_box_c.append(&card);
            }
            if ws_popup_c.is_visible() {
                ws_popup_c.set_default_size(1, 1);
            }
        }
    });

    // Audio Loop
    let vol_slider_c = vol_slider.clone();
    let vol_icon_c = vol_icon.clone();
    let popup_vol_icon_c = popup_vol_icon.clone();
    gtk4::glib::MainContext::default().spawn_local(async move {
        while let Some(data) = audio_rx.next().await {
            vol_slider_c.set_value(data.volume);
            let icon_name = if data.is_muted || data.volume == 0.0 {
                "audio-volume-muted-symbolic"
            } else if data.volume < 0.33 {
                "audio-volume-low-symbolic"
            } else if data.volume < 0.66 {
                "audio-volume-medium-symbolic"
            } else {
                "audio-volume-high-symbolic"
            };
            vol_icon_c.set_icon_name(Some(icon_name));
            popup_vol_icon_c.set_icon_name(Some(icon_name));
        }
    });

    vol_slider.connect_value_changed(move |s| {
        let _ = audio_tx.unbounded_send(s.value());
    });

    // Network Loop
    let (mut network_rx, network_tx) = NetworkService::spawn();
    let wifi_tile_c = wifi_tile.clone();
    let eth_tile_c = eth_tile.clone();
    let wifi_list_c = wifi_list.clone();
    let is_wifi_on_c = is_wifi_on.clone();
    let is_net_on_c = is_net_on.clone();
    let network_tx_loop = network_tx.clone();
    gtk4::glib::MainContext::default().spawn_local(async move {
        while let Some(data) = network_rx.next().await {
            *is_wifi_on_c.borrow_mut() = data.is_wifi_enabled;
            *is_net_on_c.borrow_mut() = data.is_networking_enabled;
            
            // Wi-Fi Tile leuchtet, wenn WLAN aktiviert ist
            if data.is_wifi_enabled {
                wifi_tile_c.add_css_class("active");
            } else {
                wifi_tile_c.remove_css_class("active");
            }

            // Ethernet Tile leuchtet, wenn wir über Kabel verbunden sind
            if data.is_ethernet_connected {
                eth_tile_c.add_css_class("active");
            } else {
                eth_tile_c.remove_css_class("active");
            }

            // Update Wi-Fi List
            // Prüfen, ob gerade jemand ein Passwort eingibt (ein Revealer offen ist)
            let mut any_expanded = false;
            let mut curr = wifi_list_c.first_child();
            while let Some(row) = curr {
                if let Some(item_box) = row.downcast_ref::<gtk4::Box>() {
                    if let Some(revealer) = item_box.last_child().and_then(|c| c.downcast::<gtk4::Revealer>().ok()) {
                        if revealer.reveals_child() {
                            any_expanded = true;
                            break;
                        }
                    }
                }
                curr = row.next_sibling();
            }

            if !any_expanded {
                while let Some(child) = wifi_list_c.first_child() {
                    wifi_list_c.remove(&child);
                }
                for ap in data.access_points {
                    let item_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
                    item_container.add_css_class("qs-wifi-item");

                    let row_btn = gtk4::Button::builder()
                        .css_classes(vec!["qs-list-row".to_string()])
                        .focusable(false) // Verhindert, dass der Button den Fokus von der Liste klaut
                        .build();
                    if ap.is_active {
                        row_btn.add_css_class("active");
                    }

                    let row_content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
                    row_content.set_margin_start(12);
                    row_content.set_margin_end(12);
                    row_content.set_margin_top(8);
                    row_content.set_margin_bottom(8);
                    
                    let icon_name = if ap.strength > 75 {
                        "network-wireless-signal-excellent-symbolic"
                    } else if ap.strength > 50 {
                        "network-wireless-signal-good-symbolic"
                    } else if ap.strength > 25 {
                        "network-wireless-signal-ok-symbolic"
                    } else {
                        "network-wireless-signal-weak-symbolic"
                    };
                    
                    row_content.append(&gtk4::Image::from_icon_name(icon_name));
                    row_content.append(&gtk4::Label::new(Some(&ap.ssid)));
                    
                    if ap.is_active {
                        let check = gtk4::Image::from_icon_name("object-select-symbolic");
                        check.set_halign(gtk4::Align::End);
                        check.set_hexpand(true);
                        row_content.append(&check);
                    } else if ap.needs_auth {
                        let lock = gtk4::Image::from_icon_name("network-wireless-encrypted-symbolic");
                        lock.set_halign(gtk4::Align::End);
                        lock.set_hexpand(true);
                        row_content.append(&lock);
                    }

                    row_btn.set_child(Some(&row_content));
                    
                    // Auth Section (Hidden by default)
                    let auth_revealer = gtk4::Revealer::new();
                    let auth_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                    auth_box.set_margin_start(12);
                    auth_box.set_margin_end(12);
                    auth_box.set_margin_bottom(12);

                    let pass_entry = gtk4::PasswordEntry::builder()
                        .placeholder_text("Password")
                        .hexpand(true)
                        .build();
                    
                    let connect_btn = gtk4::Button::builder()
                        .label("Connect")
                        .css_classes(vec!["suggested-action".to_string(), "qs-wifi-connect-btn".to_string()])
                        .build();

                    auth_box.append(&pass_entry);
                    auth_box.append(&connect_btn);
                    auth_revealer.set_child(Some(&auth_box));

                    let tx_c = network_tx_loop.clone();
                    let ap_path = ap.path.clone();
                    let is_active = ap.is_active;
                    let needs_auth = ap.needs_auth;
                    let rev = auth_revealer.clone();

                    row_btn.connect_clicked(move |_| {
                        if is_active {
                            let _ = tx_c.unbounded_send(NetworkCmd::DisconnectWifi);
                        } else if needs_auth {
                            let is_revealed = rev.reveals_child();
                            rev.set_reveal_child(!is_revealed);
                        } else {
                            let _ = tx_c.unbounded_send(NetworkCmd::ConnectToAp(ap_path.clone()));
                        }
                    });

                    let tx_connect = network_tx_loop.clone();
                    let ap_path_connect = ap.path.clone();
                    let ap_ssid_connect = ap.ssid.clone();
                    let btn_c = connect_btn.clone();
                    let pass_entry_c = pass_entry.clone();

                    // Funktion für den Verbindungsaufbau (wird von Button und Enter genutzt)
                    let do_connect = move || {
                        let password = pass_entry_c.text().to_string();
                        let spinner = gtk4::Spinner::builder()
                            .spinning(true)
                            .halign(gtk4::Align::Center)
                            .valign(gtk4::Align::Center)
                            .build();
                        btn_c.set_child(Some(&spinner));
                        btn_c.set_sensitive(false);

                        let _ = tx_connect.unbounded_send(NetworkCmd::ConnectToApWithPassword(
                            ap_path_connect.clone(),
                            ap_ssid_connect.clone(),
                            password
                        ));
                    };

                    let do_connect_btn = do_connect.clone();
                    connect_btn.connect_clicked(move |_| {
                        do_connect_btn();
                    });

                    pass_entry.connect_activate(move |_| {
                        do_connect();
                    });
                    item_container.append(&row_btn);
                    item_container.append(&auth_revealer);
                    wifi_list_c.append(&item_container);
                }
            }
        }
    });

    let network_tx_wifi = network_tx.clone();
    let is_wifi_on_toggle = is_wifi_on.clone();
    wifi_toggle.connect_clicked(move |_| {
        let current = *is_wifi_on_toggle.borrow();
        let _ = network_tx_wifi.unbounded_send(NetworkCmd::ToggleWifi(!current));
    });

    let network_tx_scan = network_tx.clone();
    if let Some(arrow) = wifi_arrow {
        let stack_clone = qs_stack.clone();
        arrow.connect_clicked(move |_| {
            stack_clone.set_visible_child_name("wifi");
            let _ = network_tx_scan.unbounded_send(NetworkCmd::ScanWifi);
        });
    }

    let network_tx_eth = network_tx.clone();
    let is_net_on_toggle = is_net_on.clone();
    eth_toggle.connect_clicked(move |_| {
        let current = *is_net_on_toggle.borrow();
        let _ = network_tx_eth.unbounded_send(NetworkCmd::ToggleNetworking(!current));
    });

    // Bluetooth Loop
    let (mut bt_rx, bt_tx) = BluetoothService::spawn();
    let bt_tile_c = bt_tile.clone();
    let is_bt_on_c = is_bt_on.clone();
    gtk4::glib::MainContext::default().spawn_local(async move {
        while let Some(data) = bt_rx.next().await {
            *is_bt_on_c.borrow_mut() = data.is_powered;
            if data.is_powered {
                bt_tile_c.add_css_class("active");
            } else {
                bt_tile_c.remove_css_class("active");
            }
        }
    });

    let is_bt_on_toggle = is_bt_on.clone();
    bt_toggle.connect_clicked(move |_| {
        let current = *is_bt_on_toggle.borrow();
        let _ = bt_tx.unbounded_send(!current);
    });

    bar_window.present();
}

// Helper: Create a Tile (Toggle Button)
fn create_tile(label: &str, icon: &str, active: bool, has_arrow: bool) -> (gtk4::Box, gtk4::Button, Option<gtk4::Button>) {
    let tile_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    tile_container.add_css_class("qs-tile");
    if active {
        tile_container.add_css_class("active");
    }

    // Main Toggle Area
    let main_btn = gtk4::Button::builder()
        .css_classes(vec!["qs-tile-main".to_string()])
        .hexpand(true)
        .build();
    
    let main_content = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    let icon_img = gtk4::Image::from_icon_name(icon);
    icon_img.set_pixel_size(18);
    let text_label = gtk4::Label::builder()
        .label(label)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["qs-tile-label".to_string()])
        .build();
    
    main_content.append(&icon_img);
    main_content.append(&text_label);
    main_btn.set_child(Some(&main_content));
    tile_container.append(&main_btn);

    // Arrow Area
    let mut arrow_btn_out = None;
    if has_arrow {
        let arrow_btn = gtk4::Button::builder()
            .icon_name("go-next-symbolic")
            .css_classes(vec!["qs-tile-arrow".to_string()])
            .build();
        
        let separator = gtk4::Separator::new(gtk4::Orientation::Vertical);
        separator.add_css_class("qs-tile-separator");
        
        tile_container.append(&separator);
        tile_container.append(&arrow_btn);
        arrow_btn_out = Some(arrow_btn);
    }

    (tile_container, main_btn, arrow_btn_out)
}

// Helper: Create a Bottom Action Button
fn create_bottom_btn(icon: &str) -> gtk4::Button {
    gtk4::Button::builder()
        .icon_name(icon)
        .css_classes(vec!["qs-bottom-btn".to_string()])
        .halign(gtk4::Align::Center) // Verhindert horizontales Strecken
        .valign(gtk4::Align::Center) // Verhindert vertikales Strecken
        .build()
}
