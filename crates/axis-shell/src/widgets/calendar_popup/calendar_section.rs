use crate::app_context::AppContext;
use axis_core::services::calendar::{CalendarEvent, DateRange};
use axis_core::services::google::GoogleAuthRegistry;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

pub fn render_calendar(
    calendar_box: &gtk4::Box,
    _range_toggle: &gtk4::Box,
    ctx: &AppContext,
    spinner: &gtk4::Spinner,
    auth_box: &gtk4::Box,
    refresh_tx: &mpsc::Sender<()>,
    selected_date: &Rc<RefCell<(i32, u32, u32)>>,
) {
    while let Some(child) = calendar_box.first_child() {
        calendar_box.remove(&child);
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
    let all_events = registry.month_events().to_vec();
    let selected_range = registry.selected_range();
    drop(registry);

    let (sel_year, sel_month, sel_day) = *selected_date.borrow();

    // Header row: "Termine" + Tag/Woche buttons side by side
    let header_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .css_classes(vec!["calendar-tasks-header-row".to_string()])
        .build();

    let label = gtk4::Label::builder()
        .label("Termine")
        .css_classes(vec!["calendar-tasks-header".to_string()])
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .build();
    header_row.append(&label);

    build_range_buttons(&header_row, ctx, selected_range, refresh_tx.clone());
    calendar_box.append(&header_row);

    let filtered = filter_events(&all_events, sel_year, sel_month, sel_day, selected_range);

    for event in &filtered {
        let row = build_event_row(event);
        calendar_box.append(&row);
    }

    if filtered.is_empty() {
        let empty_label = if selected_range == DateRange::Today {
            format!("Keine Termine am {}.{}.{}", sel_day, sel_month, sel_year)
        } else {
            "Keine Termine in dieser Woche".to_string()
        };
        let empty = gtk4::Label::builder()
            .label(&empty_label)
            .css_classes(vec!["calendar-empty".to_string()])
            .halign(gtk4::Align::Start)
            .margin_top(8)
            .build();
        calendar_box.append(&empty);
    }
}

fn filter_events(
    events: &[CalendarEvent],
    year: i32,
    month: u32,
    day: u32,
    range: DateRange,
) -> Vec<CalendarEvent> {
    let day_start = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .unwrap_or_default()
        .and_hms_opt(0, 0, 0).unwrap_or_default();

    let range_end = match range {
        DateRange::Today => {
            chrono::NaiveDate::from_ymd_opt(year, month, day)
                .unwrap_or_default()
                .and_hms_opt(23, 59, 59).unwrap_or_default()
        }
        DateRange::Week => {
            day_start + chrono::Duration::days(7)
        }
    };

    events
        .iter()
        .filter(|e| {
            let (ev_start, ev_end) = parse_event_times(e);
            ev_start <= range_end && ev_end >= day_start
        })
        .cloned()
        .collect()
}

fn parse_event_times(event: &CalendarEvent) -> (chrono::NaiveDateTime, chrono::NaiveDateTime) {
    let parse_dt = |s: &str| -> chrono::NaiveDateTime {
        let clean = s.split('+').next().unwrap_or(s).trim_end_matches('Z');
        chrono::NaiveDateTime::parse_from_str(clean, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(clean, "%Y-%m-%dT%H:%M:%S%.f"))
            .unwrap_or_else(|_| {
                chrono::NaiveDate::parse_from_str(clean, "%Y-%m-%d")
                    .unwrap_or_default()
                    .and_hms_opt(0, 0, 0).unwrap_or_default()
            })
    };

    let start = parse_dt(&event.start);
    let mut end = parse_dt(&event.end);

    if event.all_day
        && end.time() == chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_default()
    {
        end = end - chrono::Duration::seconds(1);
    }

    (start, end)
}

fn build_range_buttons(
    container: &gtk4::Box,
    ctx: &AppContext,
    selected_range: DateRange,
    refresh_tx: mpsc::Sender<()>,
) {
    let day_btn = gtk4::Button::builder()
        .label("Tag")
        .css_classes(vec!["calendar-range-btn".to_string()])
        .build();
    let week_btn = gtk4::Button::builder()
        .label("Woche")
        .css_classes(vec!["calendar-range-btn".to_string()])
        .build();

    if selected_range == DateRange::Today {
        day_btn.add_css_class("calendar-range-active");
    } else {
        week_btn.add_css_class("calendar-range-active");
    }

    let ctx_c = ctx.clone();
    let tx = refresh_tx.clone();
    day_btn.connect_clicked(move |_| {
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

    container.append(&day_btn);
    container.append(&week_btn);
}

fn build_event_row(event: &CalendarEvent) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(1)
        .css_classes(vec!["calendar-event-row".to_string()])
        .build();

    let time = gtk4::Label::builder()
        .label(&event.format_time_range())
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
        
        GoogleAuthRegistry::authenticate(&axis_core::services::google::DEFAULT_SCOPES, move |result| match result {
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
