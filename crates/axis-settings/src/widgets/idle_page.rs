use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::cell::RefCell;
use axis_domain::models::config::AxisConfig;
use crate::presentation::idle::{IdleSettingsView, IdleSettingsPresenter};
use axis_presentation::View;

const LOCK_TIMEOUTS: &[(Option<u32>, &str)] = &[
    (None, "Off"),
    (Some(60), "1 minute"),
    (Some(300), "5 minutes"),
    (Some(600), "10 minutes"),
    (Some(900), "15 minutes"),
    (Some(1800), "30 minutes"),
];

const BLANK_TIMEOUTS: &[(Option<u32>, &str)] = &[
    (None, "Off"),
    (Some(60), "1 minute"),
    (Some(300), "5 minutes"),
    (Some(600), "10 minutes"),
];

fn lock_index(value: Option<u32>) -> u32 {
    LOCK_TIMEOUTS.iter().position(|(v, _)| *v == value).unwrap_or(0) as u32
}

fn lock_value(index: u32) -> Option<u32> {
    LOCK_TIMEOUTS.get(index as usize).and_then(|(v, _)| *v)
}

fn blank_index(value: Option<u32>) -> u32 {
    BLANK_TIMEOUTS.iter().position(|(v, _)| *v == value).unwrap_or(0) as u32
}

fn blank_value(index: u32) -> Option<u32> {
    BLANK_TIMEOUTS.get(index as usize).and_then(|(v, _)| *v)
}

pub struct IdleSettingsPage {
    root: adw::ToolbarView,
    inhibit_switch: adw::SwitchRow,
    lock_combo: adw::ComboRow,
    blank_combo: adw::ComboRow,

    inhibit_callback: Rc<RefCell<Option<Box<dyn Fn(bool) + 'static>>>>,
    lock_callback: Rc<RefCell<Option<Box<dyn Fn(Option<u32>) + 'static>>>>,
    blank_callback: Rc<RefCell<Option<Box<dyn Fn(Option<u32>) + 'static>>>>,
}

impl IdleSettingsPage {
    pub fn new(_presenter: Rc<IdleSettingsPresenter>) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let preferences_page = adw::PreferencesPage::builder()
            .title("Idle")
            .icon_name("changes-prevent-symbolic")
            .build();
        toolbar_view.set_content(Some(&preferences_page));

        let behavior_group = adw::PreferencesGroup::builder()
            .title("Behavior")
            .description("Prevent automatic screen lock and blanking")
            .build();
        preferences_page.add(&behavior_group);

        let inhibit_switch = adw::SwitchRow::builder()
            .title("Idle Inhibit")
            .subtitle("When enabled, the screen will not lock or blank automatically")
            .build();
        behavior_group.add(&inhibit_switch);

        let timeouts_group = adw::PreferencesGroup::builder()
            .title("Timeouts")
            .description("How long after inactivity the screen blanks and locks. Restart required for changes to take effect.")
            .build();
        preferences_page.add(&timeouts_group);

        let lock_strings: Vec<&str> = LOCK_TIMEOUTS.iter().map(|(_, label)| *label).collect();
        let lock_combo = adw::ComboRow::builder()
            .title("Lock after")
            .model(&gtk4::StringList::new(&lock_strings))
            .build();
        timeouts_group.add(&lock_combo);

        let blank_strings: Vec<&str> = BLANK_TIMEOUTS.iter().map(|(_, label)| *label).collect();
        let blank_combo = adw::ComboRow::builder()
            .title("Blank screen after")
            .model(&gtk4::StringList::new(&blank_strings))
            .build();
        timeouts_group.add(&blank_combo);

        let page = Rc::new(Self {
            root: toolbar_view,
            inhibit_switch,
            lock_combo,
            blank_combo,
            inhibit_callback: Rc::new(RefCell::new(None)),
            lock_callback: Rc::new(RefCell::new(None)),
            blank_callback: Rc::new(RefCell::new(None)),
        });

        let cb_inhibit = page.inhibit_callback.clone();
        page.inhibit_switch.connect_active_notify(move |sw| {
            if let Some(f) = cb_inhibit.borrow().as_ref() {
                f(sw.is_active());
            }
        });

        let cb_lock = page.lock_callback.clone();
        page.lock_combo.connect_selected_notify(move |combo| {
            if let Some(f) = cb_lock.borrow().as_ref() {
                f(lock_value(combo.selected()));
            }
        });

        let cb_blank = page.blank_callback.clone();
        page.blank_combo.connect_selected_notify(move |combo| {
            if let Some(f) = cb_blank.borrow().as_ref() {
                f(blank_value(combo.selected()));
            }
        });

        page
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }
}

impl View<AxisConfig> for IdleSettingsPage {
    fn render(&self, status: &AxisConfig) {
        self.inhibit_switch.set_active(status.idle_inhibit.enabled);
        self.lock_combo.set_selected(lock_index(status.idle.lock_timeout_seconds));
        self.blank_combo.set_selected(blank_index(status.idle.blank_timeout_seconds));
    }
}

impl IdleSettingsView for IdleSettingsPage {
    fn on_inhibited_toggled(&self, f: Box<dyn Fn(bool) + 'static>) {
        *self.inhibit_callback.borrow_mut() = Some(f);
    }

    fn on_lock_timeout_changed(&self, f: Box<dyn Fn(Option<u32>) + 'static>) {
        *self.lock_callback.borrow_mut() = Some(f);
    }

    fn on_blank_timeout_changed(&self, f: Box<dyn Fn(Option<u32>) + 'static>) {
        *self.blank_callback.borrow_mut() = Some(f);
    }
}
