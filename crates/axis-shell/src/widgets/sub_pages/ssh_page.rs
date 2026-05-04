use gtk4::prelude::*;
use axis_domain::models::ssh::SshStatus;
use crate::presentation::ssh::SshPresenter;
use axis_presentation::View;
use crate::widgets::components::popup_header::PopupHeader;
use std::rc::Rc;

pub struct SshPage {
    pub container: gtk4::Box,
}

impl SshPage {
    pub fn new(presenter: Rc<SshPresenter>, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);

        let header = PopupHeader::new("SSH Sessions");
        header.connect_back(on_back);
        container.append(&header.container);

        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("rich-list");
        list_box.set_selection_mode(gtk4::SelectionMode::None);

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();
        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        let empty_label = gtk4::Label::new(Some("No active SSH sessions"));
        empty_label.add_css_class("caption");
        empty_label.set_halign(gtk4::Align::Center);
        empty_label.set_valign(gtk4::Align::Center);
        empty_label.set_vexpand(true);
        container.append(&empty_label);

        let view = Box::new(SshPageView {
            list_box,
            empty_label,
        });
        presenter.add_view(view);

        Self { container }
    }
}

struct SshPageView {
    list_box: gtk4::ListBox,
    empty_label: gtk4::Label,
}

impl View<SshStatus> for SshPageView {
    fn render(&self, status: &SshStatus) {
        while let Some(row) = self.list_box.first_child() {
            self.list_box.remove(&row);
        }

        if status.sessions.is_empty() {
            self.empty_label.set_visible(true);
            self.list_box.set_visible(false);
            return;
        }

        self.empty_label.set_visible(false);
        self.list_box.set_visible(true);

        for session in &status.sessions {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            row.add_css_class("list-row");
            row.set_margin_start(8);
            row.set_margin_end(8);

            let icon = gtk4::Image::from_icon_name("network-server-symbolic");
            icon.set_pixel_size(24);
            icon.set_margin_top(8);
            icon.set_margin_bottom(8);

            let text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
            text_box.set_hexpand(true);
            text_box.set_margin_start(8);

            let title = format!("{}@{}", session.username, session.terminal);
            let title_label = gtk4::Label::new(Some(&title));
            title_label.set_halign(gtk4::Align::Start);
            title_label.add_css_class("heading");

            let subtitle = if let Some(ref ip) = session.source_ip {
                format!("{} · {}", ip, session.connected_for)
            } else {
                session.connected_for.clone()
            };
            let subtitle_label = gtk4::Label::new(Some(&subtitle));
            subtitle_label.set_halign(gtk4::Align::Start);
            subtitle_label.add_css_class("caption");

            text_box.append(&title_label);
            text_box.append(&subtitle_label);

            row.append(&icon);
            row.append(&text_box);

            self.list_box.append(&row);
        }
    }
}
