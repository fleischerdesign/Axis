use chrono::Local;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::presentation::lock::LockView;
use crate::presentation::presenter::View;
use crate::widgets::components::blurred_picture::BlurredPicture;
use axis_domain::models::lock::LockStatus;
use axis_domain::models::power::PowerStatus;

type AuthCallback = dyn Fn(&str);

pub struct LockScreenFactory {
    wallpaper_texture: RefCell<Option<gtk4::gdk::Texture>>,
    on_auth: RefCell<Option<Rc<AuthCallback>>>,
    clock_running: Rc<Cell<bool>>,
    content_boxes: RefCell<Vec<gtk4::Box>>,
    backgrounds: RefCell<Vec<BlurredPicture>>,
    password_entries: RefCell<Vec<gtk4::PasswordEntry>>,
    error_labels: RefCell<Vec<gtk4::Label>>,
    power_status: RefCell<Option<PowerStatus>>,
}

impl LockScreenFactory {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            wallpaper_texture: RefCell::new(None),
            on_auth: RefCell::new(None),
            clock_running: Rc::new(Cell::new(false)),
            content_boxes: RefCell::new(Vec::new()),
            backgrounds: RefCell::new(Vec::new()),
            password_entries: RefCell::new(Vec::new()),
            error_labels: RefCell::new(Vec::new()),
            power_status: RefCell::new(None),
        })
    }

    pub fn set_wallpaper(&self, texture: Option<gtk4::gdk::Texture>) {
        *self.wallpaper_texture.borrow_mut() = texture.clone();
        for bg in self.backgrounds.borrow().iter() {
            bg.set_texture(texture.clone());
        }
    }

    pub fn on_authenticate(&self, callback: Rc<AuthCallback>) {
        *self.on_auth.borrow_mut() = Some(callback);
    }

    pub fn build_overlay(&self) -> gtk4::Widget {
        let overlay = gtk4::Overlay::new();

        let background = self.build_background();
        overlay.set_child(Some(&background));

        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .vexpand(true)
            .margin_bottom(60)
            .css_classes(vec!["lock-content".to_string(), "hidden".to_string()])
            .build();

        let avatar = gtk4::Image::builder()
            .icon_name("avatar-default-symbolic")
            .css_classes(vec!["lock-avatar".to_string()])
            .pixel_size(96)
            .build();
        content.append(&avatar);

        let username = Self::current_user();
        let user_label = gtk4::Label::builder()
            .label(&username)
            .css_classes(vec!["lock-username".to_string()])
            .build();
        content.append(&user_label);

        let clock_label = gtk4::Label::builder()
            .label(&Self::format_clock())
            .css_classes(vec!["lock-clock".to_string()])
            .build();
        content.append(&clock_label);

        let date_label = gtk4::Label::builder()
            .label(&Self::format_date())
            .css_classes(vec!["lock-date".to_string()])
            .build();
        content.append(&date_label);

        let battery_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .halign(gtk4::Align::Center)
            .css_classes(vec!["lock-battery".to_string()])
            .build();

        let battery_icon = gtk4::Image::from_icon_name("battery-full-symbolic");
        let battery_label = gtk4::Label::new(None);
        battery_row.append(&battery_icon);
        battery_row.append(&battery_label);

        if let Some(status) = self.power_status.borrow().as_ref() {
            battery_row.set_visible(status.has_battery);
            if status.has_battery {
                battery_label.set_text(&format!("{:.0}%", status.battery_percentage));
                battery_icon.set_icon_name(Some(Self::battery_icon_name(
                    status.battery_percentage,
                    status.is_charging,
                )));
            }
        } else {
            battery_row.set_visible(false);
        }

        content.append(&battery_row);

        let pw_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .css_classes(vec!["lock-pw-box".to_string()])
            .margin_top(24)
            .build();

        let pw_entry = gtk4::PasswordEntry::builder()
            .placeholder_text("Password")
            .css_classes(vec!["lock-pw-entry".to_string()])
            .halign(gtk4::Align::Center)
            .show_peek_icon(false)
            .build();
        pw_box.append(&pw_entry);

        let spinner = gtk4::Spinner::builder()
            .visible(false)
            .halign(gtk4::Align::Center)
            .margin_top(4)
            .build();
        pw_box.append(&spinner);

        let error_label = gtk4::Label::builder()
            .label("Wrong password")
            .css_classes(vec!["lock-error".to_string()])
            .visible(false)
            .build();
        pw_box.append(&error_label);

        content.append(&pw_box);
        overlay.add_overlay(&content);

        self.content_boxes.borrow_mut().push(content.clone());
        self.password_entries.borrow_mut().push(pw_entry.clone());
        self.error_labels.borrow_mut().push(error_label.clone());

        self.wire_auth(&pw_entry, &spinner, &error_label);
        self.start_clock_ticker(&clock_label, &date_label);

        overlay.upcast()
    }

    fn build_background(&self) -> gtk4::Widget {
        let blurred = BlurredPicture::new_empty();
        blurred.set_hexpand(true);
        blurred.set_vexpand(true);
        blurred.add_css_class("lock-bg");

        if let Some(tex) = self.wallpaper_texture.borrow().as_ref() {
            blurred.set_texture(Some(tex.clone()));
        }

        self.backgrounds.borrow_mut().push(blurred.clone());
        blurred.upcast()
    }

    fn wire_auth(
        &self,
        entry: &gtk4::PasswordEntry,
        spinner: &gtk4::Spinner,
        error_label: &gtk4::Label,
    ) {
        let on_auth = self.on_auth.borrow().clone();
        let spinner_c = spinner.clone();
        let error_c = error_label.clone();

        entry.connect_activate(move |e| {
            let password = e.text().to_string();
            if password.is_empty() {
                return;
            }
            e.set_sensitive(false);
            error_c.set_visible(false);
            spinner_c.start();
            spinner_c.set_visible(true);

            if let Some(ref callback) = on_auth {
                callback(&password);
            }
        });
    }

    fn start_clock_ticker(&self, clock: &gtk4::Label, date: &gtk4::Label) {
        if self.clock_running.get() {
            return;
        }
        self.clock_running.set(true);

        let clock_c = clock.clone();
        let date_c = date.clone();
        let running = self.clock_running.clone();
        glib::spawn_future_local(async move {
            loop {
                glib::timeout_future_seconds(1).await;
                if !running.get() {
                    break;
                }
                clock_c.set_label(&Self::format_clock());
                date_c.set_label(&Self::format_date());
            }
        });
    }

    pub fn show_content(&self) {
        for content in self.content_boxes.borrow().iter() {
            content.remove_css_class("hidden");
        }
        for entry in self.password_entries.borrow().iter() {
            entry.set_sensitive(true);
        }
        if let Some(entry) = self.password_entries.borrow().first() {
            entry.grab_focus();
        }
    }

    pub fn hide_content(&self) {
        self.clock_running.set(false);
        for content in self.content_boxes.borrow().iter() {
            content.add_css_class("hidden");
        }
        for entry in self.password_entries.borrow().iter() {
            entry.set_text("");
            entry.set_sensitive(true);
        }
        for label in self.error_labels.borrow().iter() {
            label.set_visible(false);
        }
    }

    pub fn on_auth_result(&self, success: bool) {
        for entry in self.password_entries.borrow().iter() {
            entry.set_sensitive(true);
        }

        if success {
            return;
        }

        log::warn!("[lock-screen] Auth failed");
        for entry in self.password_entries.borrow().iter() {
            entry.set_text("");
        }
        if let Some(entry) = self.password_entries.borrow().first() {
            entry.grab_focus();
            let entry = entry.clone();
            entry.add_css_class("shake");
            glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
                entry.remove_css_class("shake");
                glib::ControlFlow::Break
            });
        }
        for label in self.error_labels.borrow().iter() {
            label.set_visible(true);
            let err = label.clone();
            glib::timeout_add_local(std::time::Duration::from_secs(3), move || {
                err.set_visible(false);
                glib::ControlFlow::Break
            });
        }
    }

    pub fn update_battery(&self, status: &PowerStatus) {
        *self.power_status.borrow_mut() = Some(status.clone());
    }

    fn current_user() -> String {
        std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "user".into())
    }

    fn format_clock() -> String {
        Local::now().format("%H:%M").to_string()
    }

    fn format_date() -> String {
        Local::now().format("%A, %B %-d").to_string()
    }

    fn battery_icon_name(percentage: f64, charging: bool) -> &'static str {
        if charging {
            if percentage < 10.0 { "battery-empty-charging-symbolic" }
            else if percentage < 25.0 { "battery-caution-charging-symbolic" }
            else if percentage < 50.0 { "battery-low-charging-symbolic" }
            else if percentage < 75.0 { "battery-good-charging-symbolic" }
            else { "battery-full-charging-symbolic" }
        } else {
            if percentage < 10.0 { "battery-empty-symbolic" }
            else if percentage < 25.0 { "battery-caution-symbolic" }
            else if percentage < 50.0 { "battery-low-symbolic" }
            else if percentage < 75.0 { "battery-good-symbolic" }
            else { "battery-full-symbolic" }
        }
    }
}

impl View<LockStatus> for LockScreenFactory {
    fn render(&self, status: &LockStatus) {
        if status.is_locked {
            self.show_content();
        } else {
            self.hide_content();
        }
    }
}

impl LockView for LockScreenFactory {
    fn on_auth_result(&self, success: bool) {
        self.on_auth_result(success);
    }
}
