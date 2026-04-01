mod bindings;
mod page;
mod pages;
mod proxy;

use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::rc::Rc;

use proxy::SettingsProxy;

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

        build_window(app, &proxy);
    });

    let args: Vec<String> = std::env::args().collect();
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    application.run_with_args(&args_ref);
}

fn build_window(app: &libadwaita::Application, proxy: &Rc<SettingsProxy>) {
    use std::collections::HashMap;

    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("Axis Settings")
        .default_width(900)
        .default_height(650)
        .build();

    // ── Navigation ───────────────────────────────────────────────────────

    let sidebar_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .build();

    let nav_view = libadwaita::NavigationView::new();

    let all_pages = pages::all_pages();

    // Build pages and store references for navigation
    let mut page_map: HashMap<String, libadwaita::NavigationPage> = HashMap::new();
    let mut first = true;
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

    // Select first page by default
    if let Some(first_row) = sidebar_list.row_at_index(0) {
        sidebar_list.select_row(Some(&first_row));
    }

    // Sidebar selection → pop current, push new page
    let nav_view_c = nav_view.clone();
    let page_map = Rc::new(page_map);
    sidebar_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let id = row.widget_name();
            if !id.is_empty() {
                // Don't navigate if already on this page
                let current_tag = nav_view_c.visible_page()
                    .and_then(|p| p.tag().map(|t| t.to_string()));
                if current_tag.as_deref() == Some(id.as_str()) {
                    return;
                }
                nav_view_c.pop();
                if let Some(nav_page) = page_map.get(id.as_str()) {
                    nav_view_c.push(nav_page);
                }
            }
        }
    });

    // ── Layout ──────────────────────────────────────────────────────────

    let sidebar_scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .min_content_width(220)
        .child(&sidebar_list)
        .build();

    let sidebar_page = libadwaita::NavigationPage::new(&sidebar_scrolled, "Settings");

    let split_view = libadwaita::NavigationSplitView::builder()
        .sidebar(&sidebar_page)
        .content(&libadwaita::NavigationPage::new(&nav_view, "Settings"))
        .sidebar_width_fraction(0.3)
        .build();

    let toolbar = libadwaita::ToolbarView::builder()
        .content(&split_view)
        .build();
    toolbar.add_top_bar(&libadwaita::HeaderBar::new());

    window.set_content(Some(&toolbar));
    window.present();
}
