use libadwaita as adw;
use libadwaita::prelude::*;
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

        let clamp = adw::Clamp::builder()
            .maximum_size(760)
            .tightening_threshold(500)
            .child(&preferences_page)
            .build();

        toolbar_view.set_content(Some(&clamp));

        // 1. Hero App Header
        let hero_group = adw::PreferencesGroup::new();
        preferences_page.add(&hero_group);

        let hero_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        hero_box.set_halign(gtk4::Align::Center);
        hero_box.set_margin_top(16);
        hero_box.set_margin_bottom(16);

        let icon = gtk4::Image::from_icon_name("computer-symbolic");
        icon.set_pixel_size(96);
        hero_box.append(&icon);

        let title_label = gtk4::Label::builder()
            .label("Axis Shell")
            .css_classes(["title-1"])
            .build();
        hero_box.append(&title_label);

        let version_label = gtk4::Label::builder()
            .label(format!("Version {VERSION}"))
            .css_classes(["dim-label", "title-4"])
            .build();
        hero_box.append(&version_label);

        let desc_label = gtk4::Label::builder()
            .label("A desktop shell for the Niri Wayland compositor")
            .css_classes(["body"])
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .build();
        hero_box.append(&desc_label);

        hero_group.add(&hero_box);

        // 2. System Information Group
        let system_group = adw::PreferencesGroup::builder()
            .title("System Information")
            .build();
        preferences_page.add(&system_group);

        let os_row = adw::ActionRow::builder()
            .title("Operating System")
            .subtitle(os_name())
            .build();
        os_row.add_prefix(&gtk4::Image::from_icon_name("computer-symbolic"));
        system_group.add(&os_row);

        let kernel_row = adw::ActionRow::builder()
            .title("Kernel")
            .subtitle(kernel_version())
            .build();
        kernel_row.add_prefix(&gtk4::Image::from_icon_name("system-run-symbolic"));
        system_group.add(&kernel_row);

        let compositor_row = adw::ActionRow::builder()
            .title("Compositor")
            .subtitle("Niri")
            .build();
        compositor_row.add_prefix(&gtk4::Image::from_icon_name("video-display-symbolic"));
        system_group.add(&compositor_row);

        let session_row = adw::ActionRow::builder()
            .title("Display Server")
            .subtitle("Wayland")
            .build();
        session_row.add_prefix(&gtk4::Image::from_icon_name("video-display-symbolic"));
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
        gtk_row.add_prefix(&gtk4::Image::from_icon_name("applications-system-symbolic"));
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
        adw_row.add_prefix(&gtk4::Image::from_icon_name("applications-system-symbolic"));
        system_group.add(&adw_row);

        // 3. Links & License Group
        let links_group = adw::PreferencesGroup::builder()
            .title("Links &amp; License")
            .build();
        preferences_page.add(&links_group);

        let repo_row = adw::ActionRow::builder()
            .title("Source Code")
            .subtitle("github.com/fleischerdesign/Axis")
            .activatable(true)
            .build();
        repo_row.add_prefix(&gtk4::Image::from_icon_name("applications-system-symbolic"));
        repo_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        repo_row.connect_activated(|_| {
            let _ = gtk4::gio::AppInfo::launch_default_for_uri(
                "https://github.com/fleischerdesign/Axis",
                None::<&gtk4::gio::AppLaunchContext>,
            );
        });
        links_group.add(&repo_row);

        let issue_row = adw::ActionRow::builder()
            .title("Report an Issue")
            .subtitle("github.com/fleischerdesign/Axis/issues")
            .activatable(true)
            .build();
        issue_row.add_prefix(&gtk4::Image::from_icon_name("dialog-warning-symbolic"));
        issue_row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        issue_row.connect_activated(|_| {
            let _ = gtk4::gio::AppInfo::launch_default_for_uri(
                "https://github.com/fleischerdesign/Axis/issues",
                None::<&gtk4::gio::AppLaunchContext>,
            );
        });
        links_group.add(&issue_row);

        let license_row = adw::ActionRow::builder()
            .title("License")
            .subtitle("GPL-3.0-or-later")
            .build();
        license_row.add_prefix(&gtk4::Image::from_icon_name("dialog-information-symbolic"));
        links_group.add(&license_row);

        Rc::new(Self { root: toolbar_view })
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }
}
