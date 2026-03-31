use chrono::Local;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_session_lock as session_lock;
use log::{error, info, warn};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use axis_core::services::power::PowerData;
use axis_core::ReadOnlyHandle;
use crate::widgets::components::blurred_picture::BlurredPicture;

struct SharedState {
    instance: RefCell<Option<session_lock::Instance>>,
    locked: Cell<bool>,
    lock_confirmed: Cell<bool>,
    pending_unlock: Cell<bool>,
    password_entry: RefCell<Option<gtk4::PasswordEntry>>,
    lock_content: RefCell<Option<gtk4::Box>>,
    clock_running: Cell<bool>,
}

impl SharedState {
    fn reset(&self) {
        self.locked.set(false);
        self.lock_confirmed.set(false);
        self.pending_unlock.set(false);
        *self.password_entry.borrow_mut() = None;
        *self.lock_content.borrow_mut() = None;
    }

    fn perform_unlock(&self) {
        if let Some(inst) = self.instance.borrow().as_ref() {
            info!("[lock-screen] Unlocking session");
            inst.unlock();
            if let Some(display) = gtk4::gdk::Display::default() {
                display.sync();
            }
        }
        self.clock_running.set(false);
        self.reset();
    }
}

pub struct LockScreen {
    state: Rc<SharedState>,
    wallpaper: Option<gtk4::gdk::Texture>,
    power: ReadOnlyHandle<PowerData>,
}

impl LockScreen {
    pub fn new(wallpaper: Option<gtk4::gdk::Texture>, power: ReadOnlyHandle<PowerData>) -> Self {
        Self {
            state: Rc::new(SharedState {
                instance: RefCell::new(None),
                locked: Cell::new(false),
                lock_confirmed: Cell::new(false),
                pending_unlock: Cell::new(false),
                password_entry: RefCell::new(None),
                lock_content: RefCell::new(None),
                clock_running: Cell::new(false),
            }),
            wallpaper,
            power,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.state.locked.get()
    }

    pub fn lock_session(&self) {
        if self.state.locked.get() {
            return;
        }

        if !session_lock::is_supported() {
            warn!("[lock-screen] Session lock protocol not supported by compositor");
            return;
        }

        info!("[lock-screen] Locking session");

        let instance = session_lock::Instance::new();

        let state = self.state.clone();
        instance.connect_locked(move |_| {
            info!("[lock-screen] Session locked by compositor");
            state.lock_confirmed.set(true);

            // Fade in content
            if let Some(content) = state.lock_content.borrow().as_ref() {
                content.remove_css_class("hidden");
            }

            if let Some(entry) = state.password_entry.borrow().as_ref() {
                entry.grab_focus();
            }
            if state.pending_unlock.get() {
                info!("[lock-screen] Deferred unlock executing now");
                state.perform_unlock();
            }
        });

        let state = self.state.clone();
        instance.connect_failed(move |_| {
            warn!("[lock-screen] Lock failed — another locker holds the lock");
            *state.instance.borrow_mut() = None;
        });

        // NOTE: Do NOT connect to "unlocked" signal — it fires DURING unlock()
        // and any callback that modifies Instance state can interfere with the
        // C library's internal cleanup. State is reset manually after unlock().

        if !instance.lock() {
            error!("[lock-screen] Failed to acquire lock (immediate failure)");
            return;
        }

        *self.state.instance.borrow_mut() = Some(instance);

        let display = gtk4::gdk::Display::default().expect("No display available");
        let monitors = display.monitors();
        let mut first_entry: Option<gtk4::PasswordEntry> = None;

        for i in 0..monitors.n_items() {
            let monitor = monitors.item(i).unwrap();
            let monitor = monitor.downcast_ref::<gtk4::gdk::Monitor>().unwrap();

            let (window, entry) = self.build_lock_window();
            self.state
                .instance
                .borrow()
                .as_ref()
                .unwrap()
                .assign_window_to_monitor(&window, monitor);

            if first_entry.is_none() {
                first_entry = Some(entry);
                *self.state.password_entry.borrow_mut() = first_entry.clone();
            }
        }
    }

    // ── Build helpers ──────────────────────────────────────────────────

    fn build_background(&self) -> gtk4::Overlay {
        let overlay = gtk4::Overlay::new();

        if let Some(ref texture) = self.wallpaper {
            let blurred = BlurredPicture::new(texture);
            blurred.set_hexpand(true);
            blurred.set_vexpand(true);
            overlay.set_child(Some(&blurred));
        } else {
            let bg = gtk4::Box::builder()
                .css_classes(vec!["lock-bg".to_string()])
                .hexpand(true)
                .vexpand(true)
                .build();
            overlay.set_child(Some(&bg));
        }

        let dark_overlay = gtk4::Box::builder()
            .css_classes(vec!["lock-overlay".to_string()])
            .hexpand(true)
            .vexpand(true)
            .build();
        overlay.add_overlay(&dark_overlay);

        overlay
    }

    fn build_content(
        &self,
    ) -> (gtk4::Box, gtk4::PasswordEntry, gtk4::Spinner) {
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

        // Battery status
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

        let power = self.power.clone();
        let icon_c = battery_icon.clone();
        let label_c = battery_label.clone();
        let row_c = battery_row.clone();
        power.subscribe(move |data: &PowerData| {
            row_c.set_visible(data.has_battery);
            if data.has_battery {
                label_c.set_text(&format!("{:.0}%", data.battery_percentage));
                icon_c.set_icon_name(Some(crate::widgets::icons::battery_icon(
                    data.battery_percentage,
                    data.is_charging,
                )));
            }
        });

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

        self.start_clock_ticker(&clock_label, &date_label);
        self.wire_auth(&pw_entry, &spinner, &error_label);

        (content, pw_entry, spinner)
    }

    fn build_lock_window(&self) -> (gtk4::Window, gtk4::PasswordEntry) {
        let window = gtk4::Window::builder()
            .title("Lock Screen")
            .build();

        let overlay = self.build_background();
        let (content, entry, _spinner) = self.build_content();
        *self.state.lock_content.borrow_mut() = Some(content.clone());
        overlay.add_overlay(&content);

        window.set_child(Some(&overlay));
        (window, entry)
    }

    // ── Clock ticker ───────────────────────────────────────────────────

    fn start_clock_ticker(&self, clock: &gtk4::Label, date: &gtk4::Label) {
        let running = self.state.clock_running.clone();
        running.set(true);

        let clock_c = clock.clone();
        let date_c = date.clone();
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

    // ── Auth wiring ────────────────────────────────────────────────────

    fn wire_auth(
        &self,
        pw_entry: &gtk4::PasswordEntry,
        spinner: &gtk4::Spinner,
        error_label: &gtk4::Label,
    ) {
        let state = self.state.clone();
        let spinner = spinner.clone();
        let error_label = error_label.clone();
        let pw_entry_ref = pw_entry.clone();

        pw_entry.connect_activate(move |entry| {
            let password = entry.text().to_string();
            entry.set_sensitive(false);
            error_label.set_visible(false);

            spinner.start();
            spinner.set_visible(true);

            let (result_tx, result_rx) = std::sync::mpsc::channel::<bool>();
            std::thread::spawn(move || {
                let _ = result_tx.send(Self::pam_authenticate(&password));
            });

            let state = state.clone();
            let spinner = spinner.clone();
            let err_label = error_label.clone();
            let pw_ref = pw_entry_ref.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                if let Ok(success) = result_rx.try_recv() {
                    spinner.stop();
                    spinner.set_visible(false);

                    if success {
                        if state.lock_confirmed.get() {
                            state.perform_unlock();
                        } else {
                            info!("[lock-screen] Auth success, deferring unlock");
                            state.pending_unlock.set(true);
                        }
                    } else {
                        warn!("[lock-screen] Auth failed");
                        err_label.set_visible(true);
                        pw_ref.set_text("");
                        pw_ref.set_sensitive(true);
                        pw_ref.grab_focus();

                        // Shake animation
                        let entry_ref = pw_ref.clone();
                        entry_ref.add_css_class("shake");
                        glib::timeout_add_local(
                            std::time::Duration::from_millis(400),
                            move || {
                                entry_ref.remove_css_class("shake");
                                glib::ControlFlow::Break
                            },
                        );

                        // Auto-hide error after 3s
                        let err_hide = err_label.clone();
                        glib::timeout_add_local(
                            std::time::Duration::from_secs(3),
                            move || {
                                err_hide.set_visible(false);
                                glib::ControlFlow::Break
                            },
                        );
                    }
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            });
        });
    }

    // ── PAM ────────────────────────────────────────────────────────────

    fn pam_authenticate(password: &str) -> bool {
        let username = Self::current_user();

        let mut client = match pam::Client::with_password("login") {
            Ok(c) => c,
            Err(e) => {
                error!("[lock-screen] PAM init failed: {e}");
                return false;
            }
        };

        client
            .conversation_mut()
            .set_credentials(&username, password);

        match client.authenticate() {
            Ok(()) => true,
            Err(e) => {
                warn!("[lock-screen] PAM auth failed: {e}");
                false
            }
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────

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
}
