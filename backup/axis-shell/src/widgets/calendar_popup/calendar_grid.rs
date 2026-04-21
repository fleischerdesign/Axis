use axis_core::services::calendar::CalendarEvent;
use chrono::Datelike;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const DAY_NAMES: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];

const MONTH_NAMES: [&str; 12] = [
    "Januar", "Februar", "März", "April", "Mai", "Juni",
    "Juli", "August", "September", "Oktober", "November", "Dezember",
];

pub struct CalendarGrid {
    pub container: gtk4::Box,
    grid: gtk4::Grid,
    month_label: gtk4::Label,
    year: Rc<RefCell<i32>>,
    month: Rc<RefCell<u32>>,
    selected_day: Rc<RefCell<u32>>,
    events: Rc<RefCell<Vec<CalendarEvent>>>,
    on_day_click: Rc<RefCell<Option<Box<dyn Fn(i32, u32, u32)>>>>,
}

impl CalendarGrid {
    pub fn new() -> Self {
        let now = chrono::Local::now();
        let year = now.year();
        let month = now.month();
        let today_day = now.day();

        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .css_classes(vec!["cal-grid-wrapper".to_string()])
            .hexpand(true)
            .build();

        // ── Navigation Header ──
        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(0)
            .css_classes(vec!["cal-grid-header".to_string()])
            .hexpand(true)
            .build();

        let prev_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(vec!["cal-grid-header-btn".to_string()])
            .halign(gtk4::Align::Start)
            .build();

        let month_label = gtk4::Label::builder()
            .label(&format_month_year(month, year))
            .css_classes(vec!["cal-grid-month-label".to_string()])
            .hexpand(true)
            .halign(gtk4::Align::Center)
            .build();

        let next_btn = gtk4::Button::builder()
            .icon_name("go-next-symbolic")
            .css_classes(vec!["cal-grid-header-btn".to_string()])
            .halign(gtk4::Align::End)
            .build();

        header.append(&prev_btn);
        header.append(&month_label);
        header.append(&next_btn);
        container.append(&header);

        // ── Grid (full width, homogeneous columns) ──
        let grid = gtk4::Grid::builder()
            .column_spacing(2)
            .row_spacing(2)
            .column_homogeneous(true)
            .hexpand(true)
            .halign(gtk4::Align::Fill)
            .css_classes(vec!["cal-grid-table".to_string()])
            .build();

        container.append(&grid);

        let year_rc = Rc::new(RefCell::new(year));
        let month_rc = Rc::new(RefCell::new(month));
        let selected_day_rc = Rc::new(RefCell::new(today_day));
        let events_rc = Rc::new(RefCell::new(Vec::new()));
        let on_day_click: Rc<RefCell<Option<Box<dyn Fn(i32, u32, u32)>>>> =
            Rc::new(RefCell::new(None));

        let cal = Self {
            container,
            grid,
            month_label,
            year: year_rc.clone(),
            month: month_rc.clone(),
            selected_day: selected_day_rc.clone(),
            events: events_rc.clone(),
            on_day_click: on_day_click.clone(),
        };

        // Wire navigation
        let year_c = year_rc.clone();
        let month_c = month_rc.clone();
        let selected_c = selected_day_rc.clone();
        let events_c = events_rc.clone();
        let click_c = on_day_click.clone();
        let grid_c = cal.grid.clone();
        let label_c = cal.month_label.clone();
        prev_btn.connect_clicked(move |_| {
            let mut m = month_c.borrow_mut();
            let mut y = year_c.borrow_mut();
            if *m == 1 {
                *m = 12;
                *y -= 1;
            } else {
                *m -= 1;
            }
            let sel = *selected_c.borrow();
            label_c.set_label(&format_month_year(*m, *y));
            build_grid_cells(&grid_c, *y, *m, sel, &events_c.borrow(), &click_c);
        });

        let year_c = year_rc.clone();
        let month_c = month_rc.clone();
        let selected_c = selected_day_rc.clone();
        let events_c = events_rc.clone();
        let click_c = on_day_click.clone();
        let grid_c = cal.grid.clone();
        let label_c = cal.month_label.clone();
        next_btn.connect_clicked(move |_| {
            let mut m = month_c.borrow_mut();
            let mut y = year_c.borrow_mut();
            if *m == 12 {
                *m = 1;
                *y += 1;
            } else {
                *m += 1;
            }
            let sel = *selected_c.borrow();
            label_c.set_label(&format_month_year(*m, *y));
            build_grid_cells(&grid_c, *y, *m, sel, &events_c.borrow(), &click_c);
        });

        // Initial render
        cal.render();

        cal
    }

    pub fn set_on_day_click<F: Fn(i32, u32, u32) + 'static>(&self, cb: F) {
        *self.on_day_click.borrow_mut() = Some(Box::new(cb));
    }

    pub fn set_events(&self, events: Vec<CalendarEvent>) {
        *self.events.borrow_mut() = events;
        self.render();
    }

    pub fn select_day(&self, day: u32) {
        *self.selected_day.borrow_mut() = day;
        self.render();
    }

    pub fn selected_date(&self) -> (i32, u32, u32) {
        let y = *self.year.borrow();
        let m = *self.month.borrow();
        let d = *self.selected_day.borrow();
        (y, m, d)
    }

    pub fn navigate_to(&self, year: i32, month: u32) {
        *self.year.borrow_mut() = year;
        *self.month.borrow_mut() = month;
        self.month_label.set_label(&format_month_year(month, year));
        self.render();
    }

    fn render(&self) {
        let y = *self.year.borrow();
        let m = *self.month.borrow();
        let sel = *self.selected_day.borrow();
        build_grid_cells(
            &self.grid,
            y,
            m,
            sel,
            &self.events.borrow(),
            &self.on_day_click,
        );
    }
}

fn build_grid_cells(
    grid: &gtk4::Grid,
    year: i32,
    month: u32,
    selected_day: u32,
    events: &[CalendarEvent],
    on_click: &Rc<RefCell<Option<Box<dyn Fn(i32, u32, u32)>>>>,
) {
    // Clear existing children
    while let Some(child) = grid.first_child() {
        grid.remove(&child);
    }

    let now = chrono::Local::now();
    let today_year = now.year();
    let today_month = now.month();
    let today_day = now.day();

    // Day name headers (row 0)
    for (col, name) in DAY_NAMES.iter().enumerate() {
        let label = gtk4::Label::builder()
            .label(*name)
            .css_classes(vec!["cal-grid-day-name".to_string()])
            .halign(gtk4::Align::Center)
            .hexpand(true)
            .build();
        grid.attach(&label, col as i32, 0, 1, 1);
    }

    // Calculate first weekday (0=Monday) and days in month
    let first_weekday = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .map(|d| d.weekday().num_days_from_monday() as i32)
        .unwrap_or(0);
    let days_in_cur = days_in_month(year, month);

    // Previous month info
    let (prev_month_year, prev_month) = if month == 1 {
        (year - 1, 12u32)
    } else {
        (year, month - 1)
    };
    let days_in_prev = days_in_month(prev_month_year, prev_month);

    let mut day_counter: i32 = 1;
    let mut next_month_day: u32 = 1;
    let is_today = |y: i32, m: u32, d: u32| -> bool {
        y == today_year && m == today_month && d == today_day
    };

    for row in 1..7i32 {
        for col in 0..7i32 {
            let cell_index = (row - 1) * 7 + col;
            let is_other_month;
            let day_num: u32;
            let cell_year: i32;
            let cell_month: u32;

            if cell_index < first_weekday {
                is_other_month = true;
                day_num = (days_in_prev as i32 - first_weekday + cell_index + 1) as u32;
                cell_year = prev_month_year;
                cell_month = prev_month;
            } else if day_counter <= days_in_cur as i32 {
                is_other_month = false;
                day_num = day_counter as u32;
                cell_year = year;
                cell_month = month;
                day_counter += 1;
            } else {
                is_other_month = true;
                day_num = next_month_day;
                cell_year = if month == 12 { year + 1 } else { year };
                cell_month = if month == 12 { 1 } else { month + 1 };
                next_month_day += 1;
            }

            let cell = build_day_cell(
                day_num,
                cell_year,
                cell_month,
                is_other_month,
                is_today(cell_year, cell_month, day_num),
                !is_other_month && day_num == selected_day,
                events,
                on_click,
            );
            grid.attach(&cell, col, row, 1, 1);
        }
    }
}

fn build_day_cell(
    day: u32,
    year: i32,
    month: u32,
    is_other: bool,
    is_today: bool,
    is_selected: bool,
    events: &[CalendarEvent],
    on_click: &Rc<RefCell<Option<Box<dyn Fn(i32, u32, u32)>>>>,
) -> gtk4::Button {
    let mut css = vec!["cal-grid-cell".to_string()];
    if is_today {
        css.push("cal-grid-cell-today".to_string());
    }
    if is_selected {
        css.push("cal-grid-cell-selected".to_string());
    }
    if is_other {
        css.push("cal-grid-cell-other".to_string());
    }

    let cell_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(1)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .hexpand(true)
        .vexpand(true)
        .build();

    let day_label = gtk4::Label::builder()
        .label(&day.to_string())
        .css_classes(vec!["cal-grid-day-num".to_string()])
        .build();
    cell_box.append(&day_label);

    let btn = gtk4::Button::builder()
        .css_classes(css)
        .child(&cell_box)
        .hexpand(true)
        .build();

    // Apply event background color
    let day_events = events_for_day(year, month, day, events);
    if !day_events.is_empty() && !is_other {
        let color = color_id_to_hex(day_events[0].color_id.as_deref());
        apply_cell_background(&btn, color);
    }

    if !is_other {
        let click_c = on_click.clone();
        btn.connect_clicked(move |_| {
            if let Some(ref cb) = *click_c.borrow() {
                cb(year, month, day);
            }
        });
    }

    btn
}

fn events_for_day(year: i32, month: u32, day: u32, events: &[CalendarEvent]) -> Vec<&CalendarEvent> {
    let day_start = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .unwrap_or_default()
        .and_hms_opt(0, 0, 0).unwrap();
    let day_end = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .unwrap_or_default()
        .and_hms_opt(23, 59, 59).unwrap();

    events
        .iter()
        .filter(|e| {
            let (ev_start, ev_end) = parse_event_range(e);
            ev_start <= day_end && ev_end >= day_start
        })
        .collect()
}

fn parse_event_range(event: &CalendarEvent) -> (chrono::NaiveDateTime, chrono::NaiveDateTime) {
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

    if event.all_day && end.time() == chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_default() {
        end = end - chrono::Duration::seconds(1);
    }

    (start, end)
}

fn color_id_to_hex(color_id: Option<&str>) -> &'static str {
    match color_id {
        Some("1") => "#a4bdfc",
        Some("2") => "#7ae7bf",
        Some("3") => "#dbadff",
        Some("4") => "#ff887c",
        Some("5") => "#fbd75b",
        Some("6") => "#ffb878",
        Some("7") => "#46d6db",
        Some("8") => "#e1e1e1",
        Some("9") => "#5484ed",
        Some("10") => "#51b749",
        Some("11") => "#dc2127",
        _ => "#3584e4",
    }
}

fn apply_cell_background(btn: &gtk4::Button, color: &'static str) {
    // Convert hex to rgba with low opacity
    let (r, g, b) = hex_to_rgb(color);
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&format!(
        "button {{ background-color: rgba({}, {}, {}, 0.22); }}
         button:hover {{ background-color: rgba({}, {}, {}, 0.32); }}",
        r, g, b, r, g, b
    ));
    #[allow(deprecated)]
    {
        btn.style_context().add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (53, 132, 228) // default blue
    }
}

fn format_month_year(month: u32, year: i32) -> String {
    let name = MONTH_NAMES.get((month - 1) as usize).copied().unwrap_or("???");
    format!("{} {}", name, year)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}
