use chrono::Local;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_session_lock as session_lock;
use log::{error, info, warn};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

pub struct LockScreen {
    instance: Rc<RefCell<Option<session_lock::Instance>>>,
    locked: Rc<Cell<bool>>,
    lock_confirmed: Rc<Cell<bool>>,
    pending_unlock: Rc<Cell<bool>>,
    password_entry: Rc<RefCell<Option<gtk4::PasswordEntry>>>,
}

impl LockScreen {
    pub fn new() -> Self {
        Self {
            instance: Rc::new(RefCell::new(None)),
            locked: Rc::new(Cell::new(false)),
            lock_confirmed: Rc::new(Cell::new(false)),
            pending_unlock: Rc::new(Cell::new(false)),
            password_entry: Rc::new(RefCell::new(None)),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked.get()
    }

    /// Trigger the session lock via ext-session-lock-v1 protocol.
    /// Creates fullscreen lock windows on all connected monitors.
    pub fn lock_session(&self) {
        if self.locked.get() {
            return;
        }

        if !session_lock::is_supported() {
            warn!("[lock-screen] Session lock protocol not supported by compositor");
            return;
        }

        info!("[lock-screen] Locking session");

        let instance = session_lock::Instance::new();

        // Focus the password entry once the compositor confirms the lock.
        // If PAM auth already succeeded while waiting, unlock immediately.
        let entry_for_locked = self.password_entry.clone();
        let confirmed = self.lock_confirmed.clone();
        let pending = self.pending_unlock.clone();
        let inst_ref = self.instance.clone();
        let locked_for_deferred = self.locked.clone();
        let confirmed_for_deferred = self.lock_confirmed.clone();
        let pending_for_deferred = self.pending_unlock.clone();
        let entry_for_deferred = self.password_entry.clone();
        instance.connect_locked(move |_| {
            info!("[lock-screen] Session locked by compositor");
            confirmed.set(true);
            if let Some(entry) = entry_for_locked.borrow().as_ref() {
                entry.grab_focus();
            }
            if pending.get() {
                info!("[lock-screen] Deferred unlock executing now");
                if let Some(inst) = inst_ref.borrow().as_ref() {
                    inst.unlock();
                    if let Some(display) = gtk4::gdk::Display::default() {
                        display.sync();
                    }
                }
                locked_for_deferred.set(false);
                confirmed_for_deferred.set(false);
                pending_for_deferred.set(false);
                *entry_for_deferred.borrow_mut() = None;
            }
        });

        let inst_ref = self.instance.clone();
        instance.connect_failed(move |_| {
            warn!("[lock-screen] Lock failed — another locker holds the lock");
            *inst_ref.borrow_mut() = None;
        });

        // NOTE: Do NOT connect to "unlocked" signal — it fires DURING unlock()
        // and any callback that modifies Instance state can interfere with the
        // C library's internal cleanup. State is reset manually after unlock().

        if !instance.lock() {
            error!("[lock-screen] Failed to acquire lock (immediate failure)");
            return;
        }

        *self.instance.borrow_mut() = Some(instance);

        let display = gtk4::gdk::Display::default().expect("No display available");
        let monitors = display.monitors();
        let mut first_entry: Option<gtk4::PasswordEntry> = None;

        for i in 0..monitors.n_items() {
            let monitor = monitors.item(i).unwrap();
            let monitor = monitor.downcast_ref::<gtk4::gdk::Monitor>().unwrap();

            let (window, entry) = self.build_lock_window();
            self.instance
                .borrow()
                .as_ref()
                .unwrap()
                .assign_window_to_monitor(&window, monitor);

            if first_entry.is_none() {
                first_entry = Some(entry);
                *self.password_entry.borrow_mut() = first_entry.clone();
            }
        }
    }

    fn build_lock_window(&self) -> (gtk4::Window, gtk4::PasswordEntry) {
        let window = gtk4::Window::builder()
            .title("Lock Screen")
            .build();

        let root = gtk4::Box::builder()
            .css_classes(vec!["lock-bg".to_string()])
            .orientation(gtk4::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .valign(gtk4::Align::Fill)
            .halign(gtk4::Align::Fill)
            .build();

        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .margin_bottom(60)
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

        let error_label = gtk4::Label::builder()
            .label("Wrong password")
            .css_classes(vec!["lock-error".to_string()])
            .visible(false)
            .build();
        pw_box.append(&error_label);

        content.append(&pw_box);
        root.append(&content);

        // Clock ticker
        let clock_c = clock_label;
        let date_c = date_label;
        glib::spawn_future_local(async move {
            loop {
                glib::timeout_future_seconds(1).await;
                clock_c.set_label(&Self::format_clock());
                date_c.set_label(&Self::format_date());
            }
        });

        // PAM authentication on submit
        let inst_ref = self.instance.clone();
        let confirmed = self.lock_confirmed.clone();
        let pending = self.pending_unlock.clone();
        let locked_for_reset = self.locked.clone();
        let confirmed_for_reset = self.lock_confirmed.clone();
        let pending_for_reset = self.pending_unlock.clone();
        let entry_for_reset = self.password_entry.clone();
        let pw_entry_for_closure = pw_entry.clone();
        pw_entry.connect_activate(move |entry| {
            let password = entry.text().to_string();
            entry.set_sensitive(false);
            error_label.set_visible(false);

            let (result_tx, result_rx) = std::sync::mpsc::channel::<bool>();
            std::thread::spawn(move || {
                let _ = result_tx.send(Self::pam_authenticate(&password));
            });

            let inst = inst_ref.clone();
            let conf = confirmed.clone();
            let pend = pending.clone();
            let err_label = error_label.clone();
            let pw_entry_ref = pw_entry_for_closure.clone();
            let locked_r = locked_for_reset.clone();
            let confirmed_r = confirmed_for_reset.clone();
            let pending_r = pending_for_reset.clone();
            let entry_r = entry_for_reset.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                if let Ok(success) = result_rx.try_recv() {
                    if success {
                        if conf.get() {
                            if let Some(instance) = inst.borrow().as_ref() {
                                info!("[lock-screen] Auth success, unlocking");
                                instance.unlock();
                                if let Some(display) = gtk4::gdk::Display::default() {
                                    display.sync();
                                }
                            }
                            locked_r.set(false);
                            confirmed_r.set(false);
                            pending_r.set(false);
                            *entry_r.borrow_mut() = None;
                        } else {
                            info!("[lock-screen] Auth success, deferring unlock");
                            pend.set(true);
                        }
                    } else {
                        warn!("[lock-screen] Auth failed");
                        err_label.set_visible(true);
                        pw_entry_ref.set_text("");
                        pw_entry_ref.set_sensitive(true);
                        pw_entry_ref.grab_focus();
                    }
                    return glib::ControlFlow::Break;
                }
                glib::ControlFlow::Continue
            });
        });

        window.set_child(Some(&root));
        (window, pw_entry)
    }

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
