use super::auth_flow;
use crate::app_context::AppContext;
use crate::services::tasks::Task;
use gtk4::prelude::*;
use std::sync::mpsc;

pub fn render_tasks(
    task_list: &gtk4::Box,
    list_selector: &gtk4::Box,
    ctx: &AppContext,
    spinner: &gtk4::Spinner,
    auth_box: &gtk4::Box,
    refresh_tx: &mpsc::Sender<()>,
) {
    while let Some(child) = task_list.first_child() {
        task_list.remove(&child);
    }
    while let Some(child) = list_selector.first_child() {
        list_selector.remove(&child);
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
    let lists = registry.cached_lists().to_vec();
    let current_list_id = registry.last_list_id().unwrap_or("default").to_string();
    drop(registry);

    // ── Render List Selector ──
    let label = gtk4::Label::builder()
        .label("Aufgaben")
        .css_classes(vec!["calendar-tasks-header".to_string()])
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .build();
    list_selector.append(&label);

    if lists.len() > 1 {
        let list_names: Vec<&str> = lists.iter().map(|l| l.title.as_str()).collect();
        let model = gtk4::StringList::new(&list_names);
        let dropdown = gtk4::DropDown::builder()
            .model(&model)
            .css_classes(vec!["calendar-list-dropdown".to_string()])
            .valign(gtk4::Align::Center)
            .build();

        if let Some(idx) = lists.iter().position(|l| l.id == current_list_id) {
            dropdown.set_selected(idx as u32);
        }

        let ctx_c = ctx.clone();
        let tx_c = refresh_tx.clone();
        dropdown.connect_selected_notify(move |dd| {
            let selected = dd.selected();
            if let Some(list) = lists.get(selected as usize) {
                let reg = ctx_c.task_registry.clone();
                let list_id = list.id.clone();
                let tx = tx_c.clone();
                std::thread::spawn(move || {
                    let mut r = reg.lock().unwrap();
                    let _ = r.switch_list(&list_id);
                    let _ = tx.send(());
                });
            }
        });
        list_selector.append(&dropdown);
    }

    // ── Render Tasks ──
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

    let delete_btn = gtk4::Button::builder()
        .icon_name("user-trash-symbolic")
        .css_classes(vec!["calendar-task-delete".to_string()])
        .valign(gtk4::Align::Center)
        .build();

    row.append(&check);
    row.append(&label);
    row.append(&delete_btn);

    // Toggle handler
    let ctx_c = ctx.clone();
    let task_id = task.id.clone();
    let row_c = row.clone();
    let tx_toggle = refresh_tx.clone();
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
            let tx = tx_toggle.clone();
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

    // Delete handler
    let ctx_c = ctx.clone();
    let task_id = task.id.clone();
    let tx_delete = refresh_tx.clone();
    delete_btn.connect_clicked(move |_| {
        let is_async = {
            let registry = ctx_c.task_registry.lock().unwrap();
            registry.active().is_async()
        };

        if is_async {
            {
                let mut registry = ctx_c.task_registry.lock().unwrap();
                registry.remove_cached_task(&task_id);
            }
            let _ = tx_delete.send(());

            let reg = ctx_c.task_registry.clone();
            let tid = task_id.clone();
            let tx = tx_delete.clone();
            std::thread::spawn(move || {
                let mut r = reg.lock().unwrap();
                let list_id = r.last_list_id().unwrap_or("default").to_string();
                let _ = r.active_mut().delete_task(&list_id, &tid);
                let _ = tx.send(());
            });
        } else {
            let mut registry = ctx_c.task_registry.lock().unwrap();
            registry.optimistic_delete_task(&task_id);
            let _ = tx_delete.send(());
        }
    });

    row
}
