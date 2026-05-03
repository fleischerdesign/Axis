use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn kernel_version() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .unwrap_or_else(|_| "Unknown".to_string())
        .trim()
        .to_string()
}

fn os_name() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("PRETTY_NAME="))
                .map(|line| {
                    line.trim_start_matches("PRETTY_NAME=")
                        .trim_matches('"')
                        .to_string()
                })
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

pub struct AboutPage {
    root: adw::ToolbarView,
}

impl AboutPage {
    pub fn new() -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let preferences_page = adw::PreferencesPage::builder()
            .title("About")
            .icon_name("help-about-symbolic")
            .build();
        toolbar_view.set_content(Some(&preferences_page));

        let app_group = adw::PreferencesGroup::new();
        preferences_page.add(&app_group);

        let icon = gtk4::Image::from_icon_name("open-menu-symbolic");
        icon.set_pixel_size(128);
        app_group.add(&icon);

        let title_label = gtk4::Label::builder()
            .label("Axis")
            .css_classes(["title-4"])
            .build();
        app_group.add(&title_label);

        let version_label = gtk4::Label::builder()
            .label(format!("Version {VERSION}"))
            .css_classes(["dim-label"])
            .build();
        app_group.add(&version_label);

        let desc_label = gtk4::Label::builder()
            .label("A desktop shell for the Niri Wayland compositor")
            .css_classes(["dim-label"])
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .build();
        app_group.add(&desc_label);

        let system_group = adw::PreferencesGroup::builder()
            .title("System")
            .build();
        preferences_page.add(&system_group);

        let os_row = adw::ActionRow::builder()
            .title("Operating System")
            .subtitle(&os_name())
            .build();
        system_group.add(&os_row);

        let kernel_row = adw::ActionRow::builder()
            .title("Kernel")
            .subtitle(&kernel_version())
            .build();
        system_group.add(&kernel_row);

        let compositor_row = adw::ActionRow::builder()
            .title("Compositor")
            .subtitle("Niri")
            .build();
        system_group.add(&compositor_row);

        let session_row = adw::ActionRow::builder()
            .title("Display Server")
            .subtitle("Wayland")
            .build();
        system_group.add(&session_row);

        let gtk_version = format!(
            "{}.{}.{}",
            gtk4::major_version(),
            gtk4::minor_version(),
            gtk4::micro_version()
        );
        let gtk_row = adw::ActionRow::builder()
            .title("GTK")
            .subtitle(&gtk_version)
            .build();
        system_group.add(&gtk_row);

        let adw_version = format!(
            "{}.{}.{}",
            libadwaita::major_version(),
            libadwaita::minor_version(),
            libadwaita::micro_version()
        );
        let adw_row = adw::ActionRow::builder()
            .title("libadwaita")
            .subtitle(&adw_version)
            .build();
        system_group.add(&adw_row);

        let links_group = adw::PreferencesGroup::builder()
            .title("Links")
            .build();
        preferences_page.add(&links_group);

        let issue_link = gtk4::LinkButton::builder()
            .label("Report an Issue")
            .uri("https://github.com/anomalyco/axis/issues")
            .build();
        let issue_row = adw::ActionRow::builder()
            .title("Report an Issue")
            .build();
        issue_row.add_suffix(&issue_link);
        links_group.add(&issue_row);

        let license_row = adw::ActionRow::builder()
            .title("License")
            .subtitle("GPL-3.0")
            .build();
        links_group.add(&license_row);

        Rc::new(Self {
            root: toolbar_view,
        })
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }
}
