mod bindings;
mod continuity_proxy;
mod network_proxy;
mod bluetooth_proxy;
mod page;
mod pages;
mod proxy;
mod widgets;

use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::rc::Rc;

use proxy::SettingsProxy;
use continuity_proxy::ContinuityProxy;
use network_proxy::NetworkProxy;
use bluetooth_proxy::BluetoothProxy;
use page::SettingsPage;

fn main() {
    // Tokio runtime must live as long as the app — zbus uses it for its
    // internal I/O reactor (feature "tokio"). If it's dropped, signal
    // streams silently stop receiving data.
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let application = libadwaita::Application::builder()
        .application_id("com.github.axis.settings")
        .build();

    application.connect_activate(move |app| {
        libadwaita::init().expect("Failed to init libadwaita");

        // Load custom CSS
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        // Load config via D-Bus (blocking)
        let proxy = match rt.block_on(SettingsProxy::new()) {
            Ok(p) => Rc::new(p),
            Err(e) => {
                eprintln!("Failed to connect to Axis Shell D-Bus: {e}");
                eprintln!("Make sure the Axis shell is running.");
                std::process::exit(1);
            }
        };

        // Load continuity state via D-Bus (optional — shell may not expose it yet)
        let continuity_proxy = match rt.block_on(ContinuityProxy::new()) {
            Ok(p) => {
                log::info!("[settings] Connected to Continuity D-Bus service");
                Some(Rc::new(p))
            }
            Err(e) => {
                log::warn!("[settings] Continuity D-Bus unavailable: {e}");
                None
            }
        };

        // Load network state via D-Bus (optional)
        let network_proxy = match rt.block_on(NetworkProxy::new()) {
            Ok(p) => {
                log::info!("[settings] Connected to Network D-Bus service");
                Some(Rc::new(p))
            }
            Err(e) => {
                log::warn!("[settings] Network D-Bus unavailable: {e}");
                None
            }
        };

        // Load bluetooth state via D-Bus (optional)
        let bluetooth_proxy = match rt.block_on(BluetoothProxy::new()) {
            Ok(p) => {
                log::info!("[settings] Connected to Bluetooth D-Bus service");
                Some(Rc::new(p))
            }
            Err(e) => {
                log::warn!("[settings] Bluetooth D-Bus unavailable: {e}");
                None
            }
        };

        build_window(app, &proxy, continuity_proxy.as_ref(), network_proxy.as_ref(), bluetooth_proxy.as_ref());
    });

    let args: Vec<String> = std::env::args().collect();
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    application.run_with_args(&args_ref);
}

fn build_window(
    app: &libadwaita::Application,
    proxy: &Rc<SettingsProxy>,
    continuity: Option<&Rc<ContinuityProxy>>,
    network: Option<&Rc<NetworkProxy>>,
    bluetooth: Option<&Rc<BluetoothProxy>>,
) {
    use std::cell::RefCell;
    use std::collections::HashMap;

    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("Axis Settings")
        .default_width(900)
        .default_height(650)
        .build();

    // ── HeaderBar ────────────────────────────────────────────────────────

    let header = libadwaita::HeaderBar::new();

    let back_btn = gtk4::Button::builder()
        .icon_name("go-previous-symbolic")
        .visible(false)
        .build();
    header.pack_start(&back_btn);

    let title_label = gtk4::Label::builder()
        .label("Axis Settings")
        .css_classes(["title"])
        .build();
    header.set_title_widget(Some(&title_label));

    // ── Navigation ───────────────────────────────────────────────────────

    let sidebar_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .build();

    let nav_view = libadwaita::NavigationView::new();

    // Build pages — continuity page needs special handling for peer navigation
    let mut page_map: HashMap<String, libadwaita::NavigationPage> = HashMap::new();
    let mut first = true;

    let all_pages: Vec<Box<dyn page::SettingsPage>> = pages::all_pages_except(continuity, network, bluetooth, "continuity");

    for page in &all_pages {
        let widget = page.build(proxy);
        let nav_page = libadwaita::NavigationPage::with_tag(
            &widget, page.title(), page.id(),
        );

        if first {
            nav_view.push(&nav_page);
            first = false;
        }

        page_map.insert(page.id().to_string(), nav_page);

        let row = pages::create_sidebar_row(page.title(), page.icon(), page.id());
        sidebar_list.append(&row);
    }

    // Continuity page with peer navigation callback
    let nav_view_for_continuity = nav_view.clone();
    let continuity_page = if let Some(cp) = continuity {
        let cp_clone = cp.clone();
        let cp_for_name = cp.clone();
        pages::ContinuityPage::new(Some(&cp))
            .with_peer_callback(move |peer_id: String| {
                let cp = cp_clone.clone();
                let state = cp_for_name.state();
                let peer_name = state.peers.iter()
                    .find(|p| p.device_id == peer_id)
                    .map(|p| p.device_name.clone())
                    .unwrap_or_else(|| peer_id.clone());
                let page = pages::PeerDetailPage::new(peer_id.clone(), peer_name.clone(), cp);
                let widget = page.build();
                let nav_page = libadwaita::NavigationPage::with_tag(
                    &widget, &peer_name, "peer_detail",
                );
                nav_view_for_continuity.push(&nav_page);
            })
    } else {
        pages::ContinuityPage::new(None)
    };

    let cont_widget = continuity_page.build(proxy);
    let cont_nav_page = libadwaita::NavigationPage::with_tag(
        &cont_widget, continuity_page.title(), continuity_page.id(),
    );

    if first {
        nav_view.push(&cont_nav_page);
        first = false;
    }

    page_map.insert(continuity_page.id().to_string(), cont_nav_page);

    let row = pages::create_sidebar_row(continuity_page.title(), continuity_page.icon(), continuity_page.id());
    sidebar_list.append(&row);

    // Select first page by default
    if let Some(first_row) = sidebar_list.row_at_index(0) {
        sidebar_list.select_row(Some(&first_row));
    }

    // ── HeaderBar: update title + back button on navigation ─────────────

    let nav_view_title = nav_view.clone();
    let title_label_c = title_label.clone();
    let back_btn_c = back_btn.clone();
    nav_view.connect_visible_page_notify(move |_| {
        if let Some(page) = nav_view_title.visible_page() {
            let title = page.title().to_string();
            title_label_c.set_text(&title);

            // Show back button only for peer_detail subpages
            let is_subpage = page.tag().as_deref() == Some("peer_detail");
            back_btn_c.set_visible(is_subpage);
        }
    });

    let nav_view_back = nav_view.clone();
    back_btn.connect_clicked(move |_| {
        nav_view_back.pop();
    });

    // ── Sidebar selection → replace current page (no stack entry) ────────

    let nav_view_c = nav_view.clone();
    let page_map = Rc::new(page_map);
    sidebar_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let id = row.widget_name();
            if !id.is_empty() {
                let current_tag = nav_view_c.visible_page()
                    .and_then(|p| p.tag().map(|t| t.to_string()));
                if current_tag.as_deref() == Some(id.as_str()) {
                    return;
                }
                if let Some(nav_page) = page_map.get(id.as_str()) {
                    nav_view_c.replace(&[nav_page.clone()]);
                }
            }
        }
    });

    // ── Layout ──────────────────────────────────────────────────────────

    let sidebar_scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&sidebar_list)
        .build();
    sidebar_scrolled.set_hexpand(false);
    sidebar_scrolled.set_size_request(220, -1);

    nav_view.set_hexpand(true);
    nav_view.set_vexpand(true);

    let content_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    content_box.set_hexpand(true);
    content_box.set_vexpand(true);
    content_box.append(&sidebar_scrolled);
    content_box.append(&nav_view);

    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    main_box.set_hexpand(true);
    main_box.set_vexpand(true);
    main_box.append(&header);
    main_box.append(&content_box);

    window.set_content(Some(&main_box));
    window.present();
}
