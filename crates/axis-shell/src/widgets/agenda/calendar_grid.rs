use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use axis_domain::models::agenda::AgendaStatus;
use axis_presentation::View;
use chrono::{Datelike, Local, NaiveDate};

glib::wrapper! {
    pub struct CalendarGrid(ObjectSubclass<imp::CalendarGrid>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl CalendarGrid {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn render(&self, _status: &AgendaStatus) {
        // Here we could highlight days with events
    }
}

impl View<AgendaStatus> for CalendarGrid {
    fn render(&self, status: &AgendaStatus) {
        self.render(status);
    }
}

mod imp {
    use super::*;

    pub struct CalendarGrid {
        pub grid: gtk4::Grid,
        pub month_label: gtk4::Label,
    }

    impl Default for CalendarGrid {
        fn default() -> Self {
            Self {
                grid: gtk4::Grid::new(),
                month_label: gtk4::Label::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CalendarGrid {
        const NAME: &'static str = "AxisCalendarGrid";
        type Type = super::CalendarGrid;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for CalendarGrid {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_orientation(gtk4::Orientation::Vertical);
            obj.set_spacing(12);
            obj.set_width_request(280);

            self.month_label.add_css_class("calendar-month-label");
            obj.append(&self.month_label);

            self.grid.set_column_homogeneous(true);
            self.grid.set_row_homogeneous(true);
            self.grid.set_column_spacing(8);
            self.grid.set_row_spacing(8);
            obj.append(&self.grid);

            self.setup_calendar();
        }
    }

    impl CalendarGrid {
        fn setup_calendar(&self) {
            let now = Local::now();
            self.month_label.set_text(&now.format("%B %Y").to_string());

            let days = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
            for (i, day) in days.iter().enumerate() {
                let label = gtk4::Label::builder()
                    .label(*day)
                    .css_classes(["calendar-day-header"])
                    .build();
                self.grid.attach(&label, i as i32, 0, 1, 1);
            }

            let first_day = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
            let mut col = first_day.weekday().num_days_from_monday() as i32;
            let mut row = 1;

            let days_in_month = if now.month() == 12 {
                NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap()
            } else {
                NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap()
            }.signed_duration_since(first_day).num_days();

            for day in 1..=days_in_month {
                let label = gtk4::Label::builder()
                    .label(&day.to_string())
                    .css_classes(["calendar-day"])
                    .build();
                
                if day == now.day() as i64 {
                    label.add_css_class("today");
                }

                self.grid.attach(&label, col, row, 1, 1);
                col += 1;
                if col > 6 {
                    col = 0;
                    row += 1;
                }
            }
        }
    }

    impl WidgetImpl for CalendarGrid {}
    impl BoxImpl for CalendarGrid {}
}
