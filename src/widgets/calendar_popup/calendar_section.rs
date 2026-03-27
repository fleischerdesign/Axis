use crate::app_context::AppContext;
use crate::services::calendar::{CalendarEvent, DateRange};
use crate::services::google::{auth_flow, GoogleAuthRegistry};
use gtk4::prelude::*;
use std::sync::mpsc;

pub fn render_calendar(
    calendar_box: &gtk4::Box,
    range_toggle: &gtk4::Box,
    ctx: &AppContext,
    spinner: &gtk4::Spinner,
    auth_box: &gtk4::Box,
    refresh_tx: &mpsc::Sender<()>,
) {
    while let Some(child) = calendar_box.first_child() {
        calendar_box.remove(&child);
    }
    while let Some(child) = range_toggle.first_child() {
        range_toggle.remove(&child);
    }
    auth_box.set_visible(false);
    spinner.set_visible(false);

    let calendar_auth = GoogleAuthRegistry::load()
        .map(|r| r.is_authenticated())
        .unwrap_or(false);

    if !calendar_auth {
        show_google_auth_prompt(auth_box, ctx, refresh_tx.clone());
        return;
    }

    let registry = ctx.calendar_registry.lock().unwrap();
    let events = registry.cached_events().to_vec();
    let selected_range = registry.selected_range();
    drop(registry);

    render_range_toggle(range_toggle, ctx, selected_range, refresh_tx.clone());

    let label = gtk4::Label::builder()
        .label("Termine")
        .css_classes(vec!["calendar-tasks-header".to_string()])
        .halign(gtk4::Align::Start)
        .build();
    calendar_box.append(&label);

    for event in &events {
        let row = build_event_row(event);
        calendar_box.append(&row);
    }

    if events.is_empty() {
        let empty = gtk4::Label::builder()
            .label("Keine Termine")
            .css_classes(vec!["calendar-empty".to_string()])
            .halign(gtk4::Align::Start)
            .margin_top(8)
            .build();
        calendar_box.append(&empty);
    }
}

fn render_range_toggle(
    range_toggle: &gtk4::Box,
    ctx: &AppContext,
    selected_range: DateRange,
    refresh_tx: mpsc::Sender<()>,
) {
    let today_btn = gtk4::Button::builder()
        .label("Heute")
        .css_classes(vec!["calendar-range-btn".to_string()])
        .build();
    let week_btn = gtk4::Button::builder()
        .label("Woche")
        .css_classes(vec!["calendar-range-btn".to_string()])
        .build();

    if selected_range == DateRange::Today {
        today_btn.add_css_class("calendar-range-active");
    } else {
        week_btn.add_css_class("calendar-range-active");
    }

    let ctx_c = ctx.clone();
    let tx = refresh_tx.clone();
    today_btn.connect_clicked(move |_| {
        let mut reg = ctx_c.calendar_registry.lock().unwrap();
        reg.set_range(DateRange::Today);
        drop(reg);
        trigger_calendar_refresh(&ctx_c, tx.clone());
    });

    let ctx_c = ctx.clone();
    let tx = refresh_tx.clone();
    week_btn.connect_clicked(move |_| {
        let mut reg = ctx_c.calendar_registry.lock().unwrap();
        reg.set_range(DateRange::Week);
        drop(reg);
        trigger_calendar_refresh(&ctx_c, tx.clone());
    });

    range_toggle.append(&today_btn);
    range_toggle.append(&week_btn);
}

fn build_event_row(event: &CalendarEvent) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .css_classes(vec!["calendar-event-row".to_string()])
        .build();

    let time_label = event.format_time_range();

    let time = gtk4::Label::builder()
        .label(&time_label)
        .css_classes(vec!["calendar-event-time".to_string()])
        .halign(gtk4::Align::Start)
        .build();

    let summary = gtk4::Label::builder()
        .label(&event.summary)
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["calendar-event-summary".to_string()])
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .build();

    row.append(&time);
    row.append(&summary);

    row
}

fn show_google_auth_prompt(auth_box: &gtk4::Box, ctx: &AppContext, refresh_tx: mpsc::Sender<()>) {
    while let Some(child) = auth_box.first_child() {
        auth_box.remove(&child);
    }
    auth_box.set_visible(true);

    let label = gtk4::Label::builder()
        .label("Google Kalender & Aufgaben verbinden")
        .css_classes(vec!["auth-prompt-label".to_string()])
        .build();

    let btn = gtk4::Button::builder()
        .label("Google Anmelden")
        .css_classes(vec!["auth-prompt-btn".to_string()])
        .build();

    let ctx_c = ctx.clone();
    btn.connect_clicked(move |_| {
        let task_reg = ctx_c.task_registry.clone();
        let cal_reg = ctx_c.calendar_registry.clone();
        let tx = refresh_tx.clone();
        
        auth_flow::authenticate_async(move |result| match result {
            Ok(()) => {
                log::info!("[auth] Google auth successful, loading data...");
                
                let reg = task_reg.clone();
                let tx1 = tx.clone();
                std::thread::spawn(move || {
                    let mut r = reg.lock().unwrap();
                    let _ = r.refresh_tasks();
                    let _ = tx1.send(());
                });
                
                let reg2 = cal_reg.clone();
                std::thread::spawn(move || {
                    let mut r = reg2.lock().unwrap();
                    let _ = r.refresh_events();
                    let _ = tx.send(());
                });
            }
            Err(e) => {
                log::warn!("[auth] Google auth failed: {}", e);
            }
        });
    });

    auth_box.append(&label);
    auth_box.append(&btn);
}

fn trigger_calendar_refresh(ctx: &AppContext, tx: mpsc::Sender<()>) {
    let reg = ctx.calendar_registry.clone();
    std::thread::spawn(move || {
        let mut r = reg.lock().unwrap();
        match r.refresh_events() {
            Ok(events) => log::info!("[calendar] Refreshed {} events", events.len()),
            Err(e) => log::warn!("[calendar] Refresh failed: {}", e),
        }
        let _ = tx.send(());
    });
}