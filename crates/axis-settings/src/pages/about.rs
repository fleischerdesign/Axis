use std::rc::Rc;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::page::SettingsPage;
use crate::proxy::SettingsProxy;

pub struct AboutPage;

impl SettingsPage for AboutPage {
    fn id(&self) -> &'static str { "about" }
    fn title(&self) -> &'static str { "About" }
    fn icon(&self) -> &'static str { "help-about-symbolic" }

    fn build(&self, _proxy: &Rc<SettingsProxy>) -> gtk4::Widget {
        // ── Logo & Version ──────────────────────────────────────────────
        let logo_group = libadwaita::PreferencesGroup::builder().build();

        let logo_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        logo_box.set_margin_top(24);
        logo_box.set_margin_bottom(12);
        logo_box.set_halign(gtk4::Align::Center);

        let logo_icon = gtk4::Image::from_icon_name("preferences-system-symbolic");
        logo_icon.set_pixel_size(64);
        logo_box.append(&logo_icon);

        let name_label = gtk4::Label::builder()
            .label("Axis")
            .css_classes(["title-1"])
            .build();
        logo_box.append(&name_label);

        let version_label = gtk4::Label::builder()
            .label(&format!("Version {}", env!("CARGO_PKG_VERSION")))
            .css_classes(["dim-label"])
            .build();
        logo_box.append(&version_label);

        logo_group.add(&logo_box);

        // ── System Info ─────────────────────────────────────────────────
        let sys_group = libadwaita::PreferencesGroup::builder()
            .title("System")
            .build();

        let hostname = std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".into());

        sys_group.add(&create_info_row("Hostname", &hostname));
        sys_group.add(&create_info_row("Compositor", "niri"));
        sys_group.add(&create_info_row("Display Server", "Wayland"));

        // ── Links ───────────────────────────────────────────────────────
        let link_group = libadwaita::PreferencesGroup::builder()
            .title("Links")
            .build();

        let github_row = libadwaita::ActionRow::builder()
            .title("GitHub Repository")
            .subtitle("github.com/axis-shell/axis")
            .build();
        let github_icon = gtk4::Image::from_icon_name("web-browser-symbolic");
        github_icon.set_valign(gtk4::Align::Center);
        github_row.add_suffix(&github_icon);
        github_row.set_activatable(true);
        github_row.connect_activated(|_| {
            let _ = gtk4::show_uri(
                None::<&gtk4::Window>,
                "https://github.com/axis-shell/axis",
                0,
            );
        });
        link_group.add(&github_row);

        // ── Page Assembly ───────────────────────────────────────────────
        let page = libadwaita::PreferencesPage::new();
        page.add(&logo_group);
        page.add(&sys_group);
        page.add(&link_group);
        page.into()
    }
}

fn create_info_row(title: &str, value: &str) -> libadwaita::ActionRow {
    let row = libadwaita::ActionRow::builder()
        .title(title)
        .subtitle(value)
        .build();
    row.set_activatable(false);
    row
}
