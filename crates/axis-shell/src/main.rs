use axis_shell::Cli;
use axis_shell::composition;
use axis_shell::setup_logger;
use clap::Parser;
use gtk4::glib;
use libadwaita::prelude::*;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

fn main() -> glib::ExitCode {
    setup_logger().expect("Failed to initialize logger");
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    let cli = Cli::parse();

    let prog_name = std::env::args()
        .next()
        .unwrap_or_else(|| "axis-shell".into());

    let app = libadwaita::Application::builder()
        .application_id("org.axis.shell")
        .build();

    let theme_provider: Rc<OnceCell<Rc<gtk4::CssProvider>>> = Rc::new(OnceCell::new());
    let theme_provider_for_startup = theme_provider.clone();

    app.connect_startup(move |_| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let theme_css = Rc::new(gtk4::CssProvider::new());
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &*theme_css,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
        let _ = theme_provider_for_startup.set(theme_css);
    });

    let (prov, lock_gtk_handle) = composition::providers::setup(&cli, &rt);
    let uc = composition::use_cases::setup(&prov);
    let pres = composition::presenters::setup(&uc);

    composition::notifications::setup(&prov, &uc, &rt);

    let lock_handle = Rc::new(RefCell::new(Some(lock_gtk_handle)));

    app.connect_activate({
        let theme_provider = theme_provider.clone();
        let lock_handle = lock_handle.clone();
        move |app| {
            composition::wiring::wire(composition::wiring::WiringArgs {
                app,
                p: &prov,
                uc: &uc,
                pres: &pres,
                rt: &rt,
                theme_provider: theme_provider.clone(),
                lock_gtk_handle: lock_handle
                    .borrow_mut()
                    .take()
                    .expect("lock_gtk_handle already taken"),
                start_locked: cli.locked,
            });
        }
    });

    app.run_with_args(&[&prog_name])
}
