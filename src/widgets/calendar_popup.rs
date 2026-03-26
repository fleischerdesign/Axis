use crate::app_context::AppContext;
use crate::services::tasks::{AuthStatus, Task, TaskProvider};
use crate::shell::ShellPopup;
use crate::widgets::base::PopupBase;
use chrono::{Datelike, Local};
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, LayerShell};
use log::info;
use std::cell::RefCell;
use std::rc::Rc;

pub struct CalendarPopup {
    base: PopupBase,
    task_list: gtk4::Box,
    auth_box: gtk4::Box,
    add_entry: gtk4::Entry,
    spinner: gtk4::Spinner,
    ctx: AppContext,
}

impl ShellPopup for CalendarPopup {
    fn id(&self) -> &str {
        "calendar"
    }

    fn is_open(&self) -> bool {
        self.base.is_open.get()
    }

    fn toggle(&self) {
        if self.is_open() {
            self.close();
        } else {
            self.on_open();
            self.base.open();
        }
    }

    fn close(&self) {
        self.base
            .window
            .set_keyboard_mode(KeyboardMode::OnDemand);
        self.base.close();
    }
}

impl CalendarPopup {
    pub fn new(
        app: &libadwaita::Application,
        ctx: AppContext,
        on_state_change: impl Fn() + 'static,
    ) -> Self {
        let base = PopupBase::new(app, "AXIS Daily Panel", false);

        // Center on screen
        base.window.set_anchor(Edge::Left, false);
        base.window.set_anchor(Edge::Right, false);

        let on_change = Rc::new(on_state_change);
        base.window
            .connect_visible_notify(move |_| on_change());

        // ── Main wrapper ──
        let wrapper = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(vec!["calendar-wrapper".to_string()])
            .spacing(0)
            .build();

        // ── Date header ──
        let date_label = gtk4::Label::builder()
            .label(&format_date())
            .css_classes(vec!["calendar-date-header".to_string()])
            .halign(gtk4::Align::Start)
            .build();
        wrapper.append(&date_label);

        // ── Calendar widget ──
        let calendar = gtk4::Calendar::builder()
            .show_heading(false)
            .show_day_names(true)
            .show_week_numbers(false)
            .build();
        calendar.add_css_class("calendar-grid");
        wrapper.append(&calendar);

        // ── Separator ──
        wrapper.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));

        // ── Tasks section ──
        let tasks_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_top(4)
            .build();

        // Tasks header
        let tasks_header = gtk4::Label::builder()
            .label("Aufgaben")
            .css_classes(vec!["calendar-tasks-header".to_string()])
            .halign(gtk4::Align::Start)
            .build();
        tasks_box.append(&tasks_header);

        // Task list
        let task_list = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .build();
        tasks_box.append(&task_list);

        // Spinner (loading)
        let spinner = gtk4::Spinner::builder()
            .spinning(true)
            .visible(false)
            .halign(gtk4::Align::Center)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        tasks_box.append(&spinner);

        // Auth prompt box (hidden by default)
        let auth_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .visible(false)
            .margin_top(8)
            .build();
        tasks_box.append(&auth_box);

        // Add task row
        let add_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_top(8)
            .build();

        let add_entry = gtk4::Entry::builder()
            .placeholder_text("Neue Aufgabe...")
            .hexpand(true)
            .css_classes(vec!["calendar-add-entry".to_string()])
            .build();

        let add_btn = gtk4::Button::builder()
            .icon_name("list-add-symbolic")
            .css_classes(vec!["calendar-add-btn".to_string()])
            .build();

        add_row.append(&add_entry);
        add_row.append(&add_btn);
        tasks_box.append(&add_row);

        wrapper.append(&tasks_box);
        base.set_content(&wrapper);

        // ── Keyboard: Escape closes ──
        let base_close = base.clone();
        let key = gtk4::EventControllerKey::new();
        key.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                base_close.close();
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        base.window.add_controller(key);

        // ── Keyboard: Enter in entry adds task ──
        // (will be wired after Self creation)

        Self {
            base,
            task_list,
            auth_box,
            add_entry,
            spinner,
            ctx,
        }
    }

    fn on_open(&self) {
        self.load_tasks();

        // Wire add-task on Enter
        let ctx_c = self.ctx.clone();
        let task_list_c = self.task_list.clone();
        let spinner_c = self.spinner.clone();
        let auth_box_c = self.auth_box.clone();
        let entry_c = self.add_entry.clone();
        self.add_entry.connect_activate(move |entry| {
            let title = entry.text().to_string();
            if title.is_empty() {
                return;
            }
            entry.set_text("");

            let registry = ctx_c.task_registry.borrow();
            let provider_name = registry.active().name().to_string();
            let is_local = registry.active().is_local();
            drop(registry);

            if is_local {
                let mut registry = ctx_c.task_registry.borrow_mut();
                if let Ok(task) = registry.active_mut().add_task("default", &title) {
                    drop(registry);
                    render_tasks(&task_list_c, &ctx_c, &spinner_c, &auth_box_c);
                }
            } else {
                // Remote: run on main thread (blocks briefly)
                spinner_c.set_visible(true);
                let mut registry = ctx_c.task_registry.borrow_mut();
                let result = registry.active_mut().add_task("default", &title);
                drop(registry);
                spinner_c.set_visible(false);
                if result.is_ok() {
                    render_tasks(&task_list_c, &ctx_c, &spinner_c, &auth_box_c);
                }
            }
        });
    }

    fn load_tasks(&self) {
        render_tasks(&self.task_list, &self.ctx, &self.spinner, &self.auth_box);
    }
}

fn render_tasks(
    task_list: &gtk4::Box,
    ctx: &AppContext,
    spinner: &gtk4::Spinner,
    auth_box: &gtk4::Box,
) {
    // Clear existing
    while let Some(child) = task_list.first_child() {
        task_list.remove(&child);
    }
    auth_box.set_visible(false);
    spinner.set_visible(false);

    let mut registry = ctx.task_registry.borrow_mut();
    let provider = registry.active_mut();

    log::info!("[calendar] Provider: {}, authenticated: {}", provider.name(), provider.is_authenticated());

    match provider.auth_status() {
        AuthStatus::Authenticated => {
            // Get task lists, then fetch tasks from first list
            match provider.lists() {
                Ok(lists) => {
                    let list_id = lists.first().map(|l| l.id.as_str()).unwrap_or("default");
                    match provider.tasks(list_id) {
                        Ok(tasks) => {
                            for task in &tasks {
                                let row = build_task_row(task, ctx, task_list, spinner, auth_box);
                                task_list.append(&row);
                            }
                            if tasks.is_empty() {
                                let empty = gtk4::Label::builder()
                                    .label("Keine Aufgaben")
                                    .css_classes(vec!["calendar-empty".to_string()])
                                    .halign(gtk4::Align::Start)
                                    .margin_top(8)
                                    .build();
                                task_list.append(&empty);
                            }
                        }
                        Err(e) => {
                            log::warn!("[calendar] Failed to load tasks: {e}");
                            let empty = gtk4::Label::builder()
                                .label("Keine Aufgaben")
                                .css_classes(vec!["calendar-empty".to_string()])
                                .halign(gtk4::Align::Start)
                                .margin_top(8)
                                .build();
                            task_list.append(&empty);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("[calendar] Failed to load lists: {e}");
                    let empty = gtk4::Label::builder()
                        .label("Keine Aufgaben")
                        .css_classes(vec!["calendar-empty".to_string()])
                        .halign(gtk4::Align::Start)
                        .margin_top(8)
                        .build();
                    task_list.append(&empty);
                }
            }
        }
        AuthStatus::NeedsAuth { url, code } => {
            log::info!("[calendar] NeedsAuth: url={}, code={}", url, code);
            show_auth_prompt(auth_box, &url, &code, ctx);
        }
        AuthStatus::Failed(msg) => {
            log::warn!("[calendar] Auth failed: {msg}");
            let empty = gtk4::Label::builder()
                .label("Anmeldung fehlgeschlagen")
                .css_classes(vec!["calendar-empty".to_string()])
                .halign(gtk4::Align::Start)
                .margin_top(8)
                .build();
            task_list.append(&empty);
        }
    }
}

fn build_task_row(
    task: &Task,
    ctx: &AppContext,
    task_list: &gtk4::Box,
    spinner: &gtk4::Spinner,
    auth_box: &gtk4::Box,
) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .css_classes(vec!["calendar-task-row".to_string()])
        .build();

    if task.done {
        row.add_css_class("calendar-task-done");
    }

    let check = gtk4::CheckButton::builder()
        .active(task.done)
        .css_classes(vec!["calendar-task-check".to_string()])
        .build();

    let label = gtk4::Label::builder()
        .label(&task.title)
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["calendar-task-label".to_string()])
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .build();

    row.append(&check);
    row.append(&label);

    // Toggle handler
    let ctx_c = ctx.clone();
    let task_id = task.id.clone();
    let task_list_c = task_list.clone();
    let spinner_c = spinner.clone();
    let auth_box_c = auth_box.clone();
    let row_c = row.clone();
    check.connect_toggled(move |btn| {
        let done = btn.is_active();
        let registry = ctx_c.task_registry.borrow();
        let is_local = registry.active().is_local();
        drop(registry);

        if is_local {
            let mut registry = ctx_c.task_registry.borrow_mut();
            let _ = registry.active_mut().toggle_task("default", &task_id, done);
            drop(registry);
            if done {
                row_c.add_css_class("calendar-task-done");
            } else {
                row_c.remove_css_class("calendar-task-done");
            }
        } else {
            // Remote: run on main thread (blocks briefly)
            let mut registry = ctx_c.task_registry.borrow_mut();
            let _ = registry.active_mut().toggle_task("default", &task_id, done);
            drop(registry);
            render_tasks(&task_list_c, &ctx_c, &spinner_c, &auth_box_c);
        }
    });

    row
}

fn show_auth_prompt(
    auth_box: &gtk4::Box,
    url: &str,
    code: &str,
    ctx: &AppContext,
) {
    // Clear existing
    while let Some(child) = auth_box.first_child() {
        auth_box.remove(&child);
    }
    auth_box.set_visible(true);

    if url.is_empty() || code.is_empty() {
        // No device code yet — show "Start" button
        let info_label = gtk4::Label::builder()
            .label("Google Tasks erfordert Anmeldung")
            .css_classes(vec!["calendar-auth-info".to_string()])
            .halign(gtk4::Align::Center)
            .margin_bottom(8)
            .build();
        auth_box.append(&info_label);

        let start_btn = gtk4::Button::builder()
            .label("Anmelden")
            .css_classes(vec!["calendar-auth-btn".to_string()])
            .halign(gtk4::Align::Center)
            .build();

        let ctx_c = ctx.clone();
        start_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            btn.set_label("Warte...");

            log::info!("[calendar] Starting Google auth flow...");
            let mut registry = ctx_c.task_registry.borrow_mut();
            match registry.active_mut().authenticate() {
                Ok(_) => {
                    log::info!("[calendar] Auth complete!");
                    drop(registry);
                    // Auth succeeded — popup needs to re-render tasks
                    // We can't easily re-render here, so just update the button
                    btn.set_label("Angemeldet! Neustarten...");
                }
                Err(e) => {
                    log::warn!("[calendar] Auth failed: {e}");
                    btn.set_sensitive(true);
                    btn.set_label("Fehler — Nochmal versuchen");
                }
            }
        });
        auth_box.append(&start_btn);
    } else {
        // Device code available — show code + URL + browser button
        let info_label = gtk4::Label::builder()
            .label("Gehe zu google.com/device und gib diesen Code ein:")
            .css_classes(vec!["calendar-auth-info".to_string()])
            .halign(gtk4::Align::Center)
            .build();
        auth_box.append(&info_label);

        let code_label = gtk4::Label::builder()
            .label(code)
            .css_classes(vec!["calendar-auth-code".to_string()])
            .halign(gtk4::Align::Center)
            .build();
        auth_box.append(&code_label);

        let url_label = gtk4::Label::builder()
            .label(&format!("URL: {url}"))
            .css_classes(vec!["calendar-auth-url".to_string()])
            .halign(gtk4::Align::Center)
            .build();
        auth_box.append(&url_label);

        let open_btn = gtk4::Button::builder()
            .label("Im Browser öffnen & Anmelden")
            .css_classes(vec!["calendar-auth-btn".to_string()])
            .halign(gtk4::Align::Center)
            .margin_top(8)
            .build();

        let url_c = url.to_string();
        let ctx_c = ctx.clone();
        open_btn.connect_clicked(move |btn| {
            let _ = open::that(&url_c);
            btn.set_sensitive(false);
            btn.set_label("Warte auf Autorisierung...");

            log::info!("[calendar] Polling for Google token...");
            let mut registry = ctx_c.task_registry.borrow_mut();
            match registry.active_mut().authenticate() {
                Ok(_) => {
                    log::info!("[calendar] Auth complete!");
                    btn.set_label("Angemeldet!");
                }
                Err(e) => {
                    log::warn!("[calendar] Auth failed: {e}");
                    btn.set_sensitive(true);
                    btn.set_label("Fehler — Nochmal versuchen");
                }
            }
        });
        auth_box.append(&open_btn);
    }
}

fn format_date() -> String {
    let now = Local::now();
    // German-style date: "Montag, 24. März 2026"
    let weekday = match now.weekday() {
        chrono::Weekday::Mon => "Montag",
        chrono::Weekday::Tue => "Dienstag",
        chrono::Weekday::Wed => "Mittwoch",
        chrono::Weekday::Thu => "Donnerstag",
        chrono::Weekday::Fri => "Freitag",
        chrono::Weekday::Sat => "Samstag",
        chrono::Weekday::Sun => "Sonntag",
    };
    let month = match now.month() {
        1 => "Januar",
        2 => "Februar",
        3 => "März",
        4 => "April",
        5 => "Mai",
        6 => "Juni",
        7 => "Juli",
        8 => "August",
        9 => "September",
        10 => "Oktober",
        11 => "November",
        12 => "Dezember",
        _ => unreachable!(),
    };
    format!("{}, {}. {} {}", weekday, now.day(), month, now.year())
}
