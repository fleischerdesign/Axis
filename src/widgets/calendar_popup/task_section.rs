use super::auth_flow;
use crate::app_context::AppContext;
use crate::services::tasks::Task;
use gtk4::prelude::*;
use std::sync::mpsc;

pub fn render_tasks(
    task_list: &gtk4::Box,
    ctx: &AppContext,
    spinner: &gtk4::Spinner,
    auth_box: &gtk4::Box,
    refresh_tx: &mpsc::Sender<()>,
) {
    while let Some(child) = task_list.first_child() {
        task_list.remove(&child);
    }
    auth_box.set_visible(false);
    spinner.set_visible(false);

    let registry = ctx.task_registry.lock().unwrap();

    if !registry.active().is_authenticated() {
        drop(registry);
        auth_flow::show_auth_prompt(auth_box, ctx);
        return;
    }

    let tasks = registry.cached_tasks().to_vec();
    drop(registry);

    for task in &tasks {
        let row = build_task_row(task, ctx, refresh_tx.clone());
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

fn build_task_row(
    task: &Task,
    ctx: &AppContext,
    refresh_tx: mpsc::Sender<()>,
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
    let row_c = row.clone();
    check.connect_toggled(move |btn| {
        let done = btn.is_active();
        let is_async = {
            let registry = ctx_c.task_registry.lock().unwrap();
            registry.active().is_async()
        };

        if done {
            row_c.add_css_class("calendar-task-done");
        } else {
            row_c.remove_css_class("calendar-task-done");
        }

        if is_async {
            // Async: optimistic UI + background API call
            {
                let mut registry = ctx_c.task_registry.lock().unwrap();
                registry.update_cached_task(&task_id, done);
            }
            let reg = ctx_c.task_registry.clone();
            let tid = task_id.clone();
            let tx = refresh_tx.clone();
            std::thread::spawn(move || {
                let mut r = reg.lock().unwrap();
                let list_id = r.last_list_id().unwrap_or("default").to_string();
                let _ = r.active_mut().toggle_task(&list_id, &tid, done);
                let _ = tx.send(());
            });
        } else {
            // Sync: update directly
            let mut registry = ctx_c.task_registry.lock().unwrap();
            registry.optimistic_toggle_task(&task_id, done);
        }
    });

    row
}
