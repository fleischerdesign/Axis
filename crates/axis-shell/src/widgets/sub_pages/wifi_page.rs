use gtk4::prelude::*;
use axis_domain::models::network::{NetworkStatus, AccessPoint};
use axis_presentation::View;
use crate::presentation::network::{NetworkPresenter, NetworkView};
use crate::widgets::components::list_row::ListRow;
use crate::widgets::components::popup_header::PopupHeader;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct RowEntry {
    list_box_row: gtk4::ListBoxRow,
    outer: gtk4::Box,
    list_row: ListRow,
    auth_revealer: gtk4::Revealer,
    connect_btn: gtk4::Button,
}

fn wifi_signal_icon(strength: u8) -> &'static str {
    if strength > 75 {
        "network-wireless-signal-excellent-symbolic"
    } else if strength > 50 {
        "network-wireless-signal-good-symbolic"
    } else if strength > 25 {
        "network-wireless-signal-ok-symbolic"
    } else {
        "network-wireless-signal-weak-symbolic"
    }
}

fn ap_icon(ap: &AccessPoint) -> &'static str {
    if ap.is_active {
        "network-wireless-connected-symbolic"
    } else if ap.needs_auth {
        "network-wireless-encrypted-symbolic"
    } else {
        wifi_signal_icon(ap.strength)
    }
}

fn ap_subtitle(ap: &AccessPoint) -> Option<String> {
    if ap.is_active {
        Some(format!("Connected · {}%", ap.strength))
    } else if ap.needs_auth {
        Some(format!("Secured · {}%", ap.strength))
    } else if ap.strength > 0 {
        Some(format!("{}%", ap.strength))
    } else {
        None
    }
}

pub struct WifiPage {
    pub container: gtk4::Box,
    _presenter: Rc<NetworkPresenter>,
}

impl WifiPage {
    pub fn new(presenter: Rc<NetworkPresenter>, on_back: impl Fn() + 'static) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);

        let spinner = gtk4::Spinner::builder().spinning(true).build();
        let header = PopupHeader::with_spinner("Wi-Fi Networks", &spinner);
        container.append(&header.container);

        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list".to_string()])
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .min_content_height(200)
            .build();
        scrolled.set_child(Some(&list));
        container.append(&scrolled);

        let on_back = Rc::new(on_back);
        header.connect_back(move || on_back());

        let presenter_c = presenter.clone();
        let list_c = list.clone();
        let rows: Rc<RefCell<HashMap<String, RowEntry>>> = Rc::new(RefCell::new(HashMap::new()));
        let rows_c = rows.clone();
        let spinner_c = spinner.clone();

        let view = Box::new(WifiPageView {
            rows: rows_c,
            list: list_c,
            spinner: spinner_c,
            presenter: presenter_c,
        });
        presenter.add_view(view);

        Self { container, _presenter: presenter }
    }
}

struct WifiPageView {
    rows: Rc<RefCell<HashMap<String, RowEntry>>>,
    list: gtk4::ListBox,
    spinner: gtk4::Spinner,
    presenter: Rc<NetworkPresenter>,
}

impl View<NetworkStatus> for WifiPageView {
    fn render(&self, status: &NetworkStatus) {
        self.spinner.set_spinning(status.is_scanning);

        let mut rows = self.rows.borrow_mut();

        let new_ids: std::collections::HashSet<&str> = status
            .access_points
            .iter()
            .map(|ap| ap.id.as_str())
            .collect();

        let stale: Vec<String> = rows
            .keys()
            .filter(|id| !new_ids.contains(id.as_str()))
            .cloned()
            .collect();
        for id in stale {
            if let Some(entry) = rows.remove(&id) {
                self.list.remove(&entry.list_box_row);
            }
        }

        for ap in &status.access_points {
            let icon = ap_icon(ap);
            let subtitle = ap_subtitle(ap);

            if let Some(entry) = rows.get(&ap.id) {
                entry.list_row.set_icon(icon);
                entry.list_row.set_active(ap.is_active);
                entry.list_row.set_subtitle(subtitle.as_deref());
                if ap.is_active && entry.auth_revealer.reveals_child() {
                    entry.auth_revealer.set_reveal_child(false);
                    entry.outer.remove_css_class("expanded");
                }
                if ap.is_active {
                    entry.connect_btn.set_child(None::<&gtk4::Widget>);
                    entry.connect_btn.set_label("Connect");
                    entry.connect_btn.set_sensitive(true);
                }
                continue;
            }

            let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

            let list_row = ListRow::new(&ap.ssid, icon);
            list_row.set_subtitle(subtitle.as_deref());
            list_row.set_active(ap.is_active);
            outer.append(&list_row.container);

            let auth_revealer = gtk4::Revealer::builder()
                .transition_type(gtk4::RevealerTransitionType::SlideDown)
                .transition_duration(200)
                .build();

            let connect_btn = gtk4::Button::builder()
                .label("Connect")
                .css_classes(vec!["suggested-action".to_string()])
                .build();
            connect_btn.set_visible(false);

            if ap.needs_auth {
                let auth_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
                auth_box.set_margin_top(4);
                auth_box.set_margin_start(12);
                auth_box.set_margin_end(12);

                let pass_entry = gtk4::PasswordEntry::builder()
                    .placeholder_text("Password")
                    .hexpand(true)
                    .show_peek_icon(true)
                    .build();

                let cbtn = connect_btn.clone();
                let pres = self.presenter.clone();
                let ap_id = ap.id.clone();
                let pass_e = pass_entry.clone();
                connect_btn.connect_clicked(move |_| {
                    let password = pass_e.text().to_string();
                    if !password.is_empty() {
                        let spinner = gtk4::Spinner::builder()
                            .spinning(true)
                            .build();
                        cbtn.set_child(Some(&spinner));
                        cbtn.set_sensitive(false);
                        pres.connect_to_ap(ap_id.clone(), Some(password));
                    }
                });

                auth_box.append(&pass_entry);
                auth_box.append(&connect_btn);
                auth_revealer.set_child(Some(&auth_box));
            }

            outer.append(&auth_revealer);

            let list_box_row = gtk4::ListBoxRow::builder()
                .css_classes(vec!["qs-wifi-item".to_string()])
                .selectable(false)
                .activatable(false)
                .child(&outer)
                .build();

            let pres = self.presenter.clone();
            let ap_id = ap.id.clone();
            let needs_auth = ap.needs_auth;
            let rev = auth_revealer.clone();
            let outer_c = outer.clone();
            let active = ap.is_active;

            let gesture = gtk4::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                if active {
                    pres.disconnect_wifi();
                } else if needs_auth {
                    let open = rev.reveals_child();
                    rev.set_reveal_child(!open);
                    if open {
                        outer_c.remove_css_class("expanded");
                    } else {
                        outer_c.add_css_class("expanded");
                    }
                } else {
                    pres.connect_to_ap(ap_id.clone(), None);
                }
            });
            outer.add_controller(gesture);

            rows.insert(
                ap.id.clone(),
                RowEntry {
                    list_box_row,
                    outer,
                    list_row,
                    auth_revealer,
                    connect_btn,
                },
            );
            self.list.append(&rows[&ap.id].list_box_row);
        }
    }
}

impl NetworkView for WifiPageView {
    fn on_scan_requested(&self, _f: Box<dyn Fn() + 'static>) {}
    fn on_connect_to_ap(&self, _f: Box<dyn Fn(String, Option<String>) + 'static>) {}
    fn on_disconnect_wifi(&self, _f: Box<dyn Fn() + 'static>) {}
}
