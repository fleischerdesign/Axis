mod bindings;
mod config;
mod page;
mod pages;
mod proxy;

use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::rc::Rc;

use page::SettingsPage;
use proxy::SettingsProxy;

fn main() {
    let application = libadwaita::Application::builder()
        .application_id("com.github.axis.settings")
        .build();

    application.connect_activate(|app| {
        // Load CSS
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
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
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
    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("Axis Settings")
        .default_width(900)
        .default_height(650)
        .build();

    // ── Navigation Split View ───────────────────────────────────────────

    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::Crossfade)
        .transition_duration(200)
        .hexpand(true)
        .vexpand(true)
        .build();

    let sidebar_list = gtk4::ListBox::builder()
        .css_classes(vec!["navigation-sidebar".to_string()])
        .selection_mode(gtk4::SelectionMode::Single)
        .build();

    let all_pages = pages::all_pages();

    for page in &all_pages {
        // Add page to stack
        let widget = page.build(proxy);
        stack.add_named(&widget, Some(page.id()));

        // Add sidebar row
        let row = pages::create_sidebar_row(page.title(), page.icon(), page.id());
        sidebar_list.append(&row);
    }

    // Select first page by default
    if let Some(first_row) = sidebar_list.row_at_index(0) {
        sidebar_list.select_row(Some(&first_row));
    }

    // Sidebar selection → stack page
    let stack_c = stack.clone();
    sidebar_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let id = row.widget_name();
            if !id.is_empty() {
                stack_c.set_visible_child_name(id.as_str());
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

    let split = gtk4::Paned::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .position(220)
        .wide_handle(true)
        .build();
    split.set_start_child(Some(&sidebar_scrolled));
    split.set_end_child(Some(&stack));

    // Header bar
    let header = libadwaita::HeaderBar::new();

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content.append(&header);
    content.append(&split);

    window.set_content(Some(&content));
    window.present();
}
