use libadwaita::prelude::*;
use axis_domain::models::agenda::AgendaStatus;
use axis_domain::models::calendar::CalendarEvent;
use axis_presentation::View;
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const DAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

#[derive(Clone)]
pub struct CalendarGrid {
    pub container: gtk4::Box,
    grid: gtk4::Grid,
    month_label: gtk4::Label,
    year: Rc<Cell<i32>>,
    month: Rc<Cell<u32>>,
    selected_day: Rc<Cell<u32>>,
    events: Rc<RefCell<Vec<CalendarEvent>>>,
}

impl CalendarGrid {
    pub fn new() -> Self {
        let now = Local::now();

        let grid = gtk4::Grid::new();
        grid.set_column_homogeneous(true);
        grid.set_row_homogeneous(true);
        grid.set_column_spacing(2);
        grid.set_row_spacing(2);

        let month_label = gtk4::Label::new(None);
        month_label.set_hexpand(true);
        month_label.add_css_class("calendar-month-label");

        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_bottom(8)
            .build();
        let prev = gtk4::Button::builder().icon_name("go-previous-symbolic").css_classes(["flat"]).build();
        let next = gtk4::Button::builder().icon_name("go-next-symbolic").css_classes(["flat"]).build();
        header.append(&prev);
        header.append(&month_label);
        header.append(&next);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        container.set_width_request(240);
        container.append(&header);
        container.append(&grid);

        let cal = Self {
            container,
            grid,
            month_label,
            year: Rc::new(Cell::new(now.year())),
            month: Rc::new(Cell::new(now.month())),
            selected_day: Rc::new(Cell::new(now.day())),
            events: Rc::new(RefCell::new(Vec::new())),
        };

        let cal_c = cal.clone();
        prev.connect_clicked(move |_| {
            let m = cal_c.month.get();
            let y = cal_c.year.get();
            if m == 1 { cal_c.month.set(12); cal_c.year.set(y - 1); }
            else { cal_c.month.set(m - 1); }
            cal_c.update_grid();
        });

        let cal_c = cal.clone();
        next.connect_clicked(move |_| {
            let m = cal_c.month.get();
            let y = cal_c.year.get();
            if m == 12 { cal_c.month.set(1); cal_c.year.set(y + 1); }
            else { cal_c.month.set(m + 1); }
            cal_c.update_grid();
        });

        cal.update_grid();
        cal
    }

    fn update_grid(&self) {
        let y = self.year.get();
        let m = self.month.get();
        let sel = self.selected_day.get();
        let events = self.events.borrow();

        self.month_label.set_text(&format!("{} {}", MONTH_NAMES[(m - 1) as usize], y));

        while let Some(child) = self.grid.first_child() {
            self.grid.remove(&child);
        }

        for (i, name) in DAY_NAMES.iter().enumerate() {
            let label = gtk4::Label::builder()
                .label(*name)
                .css_classes(["calendar-day-header"])
                .build();
            self.grid.attach(&label, i as i32, 0, 1, 1);
        }

        let now = Local::now();
        let first_weekday = NaiveDate::from_ymd_opt(y, m, 1)
            .map(|d| d.weekday().num_days_from_monday() as i32)
            .unwrap_or(0);

        let days_in_cur = days_in_month(y, m);
        let (prev_y, prev_m) = if m == 1 { (y - 1, 12) } else { (y, m - 1) };
        let days_in_prev = days_in_month(prev_y, prev_m);

        let mut day_counter = 1;
        let mut next_month_day = 1;

        for row in 1..7 {
            for col in 0..7 {
                let cell_idx = (row - 1) * 7 + col;
                let day_num;
                let is_other;
                let cell_y;
                let cell_m;

                if cell_idx < first_weekday {
                    is_other = true;
                    day_num = days_in_prev - (first_weekday - cell_idx - 1) as u32;
                    cell_y = prev_y;
                    cell_m = prev_m;
                } else if day_counter <= days_in_cur {
                    is_other = false;
                    day_num = day_counter as u32;
                    cell_y = y;
                    cell_m = m;
                    day_counter += 1;
                } else {
                    is_other = true;
                    day_num = next_month_day;
                    cell_y = if m == 12 { y + 1 } else { y };
                    cell_m = if m == 12 { 1 } else { m + 1 };
                    next_month_day += 1;
                }

                let btn = self.create_day_btn(day_num, cell_y, cell_m, is_other, sel, &now, &events);
                self.grid.attach(&btn, col, row, 1, 1);
            }
        }
    }

    fn create_day_btn(&self, day: u32, y: i32, m: u32, is_other: bool, sel: u32, now: &chrono::DateTime<Local>, events: &[CalendarEvent]) -> gtk4::Button {
        let mut css = vec!["calendar-day".to_string()];
        if !is_other && day == now.day() && m == now.month() && y == now.year() {
            css.push("today".to_string());
        }
        if !is_other && day == sel {
            css.push("selected".to_string());
        }
        if is_other {
            css.push("other-month".to_string());
        }

        let btn = gtk4::Button::builder()
            .label(&day.to_string())
            .css_classes(css)
            .build();

        if !is_other {
            let day_events = events_for_day(y, m, day, events);
            if let Some(event) = day_events.first() {
                let color = color_id_to_hex(event.color_id.as_deref());
                apply_btn_bg(&btn, color);
            }

            let cal = self.clone();
            btn.connect_clicked(move |_| {
                cal.selected_day.set(day);
                cal.update_grid();
            });
        }

        btn
    }
}

impl View<AgendaStatus> for CalendarGrid {
    fn render(&self, status: &AgendaStatus) {
        *self.events.borrow_mut() = status.events.clone();
        self.update_grid();
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 29 } else { 28 },
        _ => 30,
    }
}

fn events_for_day(y: i32, m: u32, d: u32, events: &[CalendarEvent]) -> Vec<&CalendarEvent> {
    let day_start = NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let day_end = NaiveDate::from_ymd_opt(y, m, d).unwrap().and_hms_opt(23, 59, 59).unwrap();
    events.iter().filter(|e| {
        let (s, e) = parse_range(e);
        s <= day_end && e >= day_start
    }).collect()
}

fn parse_range(e: &CalendarEvent) -> (NaiveDateTime, NaiveDateTime) {
    let p = |s: &str| NaiveDateTime::parse_from_str(s.split('+').next().unwrap().trim_end_matches('Z'), "%Y-%m-%dT%H:%M:%S").unwrap_or_else(|_| {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap_or_default().and_hms_opt(0, 0, 0).unwrap()
    });
    (p(&e.start), p(&e.end))
}

fn color_id_to_hex(id: Option<&str>) -> &'static str {
    match id {
        Some("1") => "#a4bdfc", Some("2") => "#7ae7bf", Some("3") => "#dbadff", Some("4") => "#ff887c",
        Some("5") => "#fbd75b", Some("6") => "#ffb878", Some("7") => "#46d6db", Some("8") => "#e1e1e1",
        Some("9") => "#5484ed", Some("10") => "#51b749", Some("11") => "#dc2127", _ => "#3584e4",
    }
}

fn apply_btn_bg(btn: &gtk4::Button, hex: &str) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&format!("button {{ background-color: alpha({}, 0.2); border-color: alpha({}, 0.4); }}", hex, hex));
    #[allow(deprecated)]
    btn.style_context().add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
}
