mod auth_flow;
mod date;
mod task_section;

use crate::app_context::AppContext;
use crate::shell::PopupExt;
use crate::widgets::base::PopupBase;
use gtk4::prelude::*;
use gtk4_layer_shell::{KeyboardMode, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

pub struct CalendarPopup {
    base: PopupBase,
    task_list: gtk4::Box,
    auth_box: gtk4::Box,
    add_entry: gtk4::Entry,
    spinner: gtk4::Spinner,
    ctx: AppContext,
    refresh_tx: mpsc::Sender<()>,
    refresh_rx: Rc<RefCell<Option<mpsc::Receiver<()>>>>,
    refresh_source: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
}

impl PopupExt for CalendarPopup {
    fn id(&self) -> &str {
        "calendar"
    }

    fn base(&self) -> &PopupBase {
        &self.base
    }

    fn on_open(&self) {
        task_section::render_tasks(
            &self.task_list,
            &self.ctx,
            &self.spinner,
            &self.auth_box,
            &self.refresh_tx,
        );

        // Start refresh timer once (survives popup close/open cycles)
        if self.refresh_source.borrow().is_some() {
            self.trigger_background_refresh();
            return;
        }

        // Take receiver (only done once)
        let rx = self.refresh_rx.borrow_mut().take()
            .expect("refresh_rx already taken");
        let tl = self.task_list.clone();
        let ctx = self.ctx.clone();
        let sp = self.spinner.clone();
        let ab = self.auth_box.clone();
        let tx = self.refresh_tx.clone();
        let source = self.refresh_source.clone();
        let base_is_open = self.base.is_open.clone();
        let src = gtk4::glib::timeout_add_local(
            std::time::Duration::from_millis(300),
            move || {
                if rx.try_recv().is_ok() && base_is_open.get() {
                    task_section::render_tasks(&tl, &ctx, &sp, &ab, &tx);
                }
                gtk4::glib::ControlFlow::Continue
            },
        );
        *source.borrow_mut() = Some(src);

        self.trigger_background_refresh();
        self.wire_add_task();
    }

    fn on_close(&self) {
        self.base.window.set_keyboard_mode(KeyboardMode::OnDemand);
    }
}

impl CalendarPopup {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Self {
        let base = PopupBase::new_centered(app, "AXIS Daily Panel");

        // ── Main wrapper ──
        let wrapper = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(vec!["calendar-wrapper".to_string()])
            .spacing(0)
            .build();

        // ── Date header ──
        let date_label = gtk4::Label::builder()
            .label(&date::format_date())
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

        let tasks_header = gtk4::Label::builder()
            .label("Aufgaben")
            .css_classes(vec!["calendar-tasks-header".to_string()])
            .halign(gtk4::Align::Start)
            .build();
        tasks_box.append(&tasks_header);

        let task_list = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .build();
        tasks_box.append(&task_list);

        let spinner = gtk4::Spinner::builder()
            .spinning(true)
            .visible(false)
            .halign(gtk4::Align::Center)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        tasks_box.append(&spinner);

        let auth_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .visible(false)
            .margin_top(8)
            .build();
        tasks_box.append(&auth_box);

        // ── Add task row ──
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

        let (refresh_tx, refresh_rx) = mpsc::channel::<()>();

        Self {
            base,
            task_list,
            auth_box,
            add_entry,
            spinner,
            ctx,
            refresh_tx,
            refresh_rx: Rc::new(RefCell::new(Some(refresh_rx))),
            refresh_source: Rc::new(RefCell::new(None)),
        }
    }

    fn trigger_background_refresh(&self) {
        let reg = self.ctx.task_registry.clone();
        let tx = self.refresh_tx.clone();
        std::thread::spawn(move || {
            let mut r = reg.lock().unwrap();
            match r.refresh_tasks() {
                Ok(tasks) => log::info!("[calendar] Refreshed {} tasks", tasks.len()),
                Err(e) => log::warn!("[calendar] Refresh failed: {e}"),
            }
            let _ = tx.send(());
        });
    }

    fn wire_add_task(&self) {
        let ctx_c = self.ctx.clone();
        let task_list_c = self.task_list.clone();
        let spinner_c = self.spinner.clone();
        let auth_box_c = self.auth_box.clone();
        let tx_c = self.refresh_tx.clone();

        self.add_entry.connect_activate(move |entry| {
            let title = entry.text().to_string();
            if title.is_empty() {
                return;
            }
            entry.set_text("");

            let is_async = {
                let registry = ctx_c.task_registry.lock().unwrap();
                registry.active().is_async()
            };

            if is_async {
                {
                    let mut registry = ctx_c.task_registry.lock().unwrap();
                    registry.optimistic_add_task(&title);
                }
                task_section::render_tasks(&task_list_c, &ctx_c, &spinner_c, &auth_box_c, &tx_c);

                let reg = ctx_c.task_registry.clone();
                let title_c = title.clone();
                std::thread::spawn(move || {
                    let mut r = reg.lock().unwrap();
                    let list_id = r.last_list_id().unwrap_or("default").to_string();
                    let _ = r.active_mut().add_task(&list_id, &title_c);
                    let _ = r.refresh_tasks();
                });
            } else {
                {
                    let mut registry = ctx_c.task_registry.lock().unwrap();
                    registry.optimistic_add_task(&title);
                }
                task_section::render_tasks(&task_list_c, &ctx_c, &spinner_c, &auth_box_c, &tx_c);
            }
        });
    }
}
