use gtk4::prelude::*;
use gtk4::glib;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use crate::proxy::SettingsProxy;

pub fn create_updating_guard() -> Rc<Cell<bool>> {
    Rc::new(Cell::new(false))
}

pub fn connect_debounced_slider(
    slider: &gtk4::Scale,
    proxy: &Rc<SettingsProxy>,
    updating: &Rc<Cell<bool>>,
    apply: Rc<dyn Fn(&SettingsProxy, f64)>,
) {
    let proxy_c = proxy.clone();
    let updating_c = updating.clone();
    let debounce: Rc<std::cell::RefCell<Option<glib::SourceId>>> = Rc::new(std::cell::RefCell::new(None));

    slider.connect_value_changed(move |s| {
        if updating_c.get() { return; }
        if let Some(id) = debounce.borrow_mut().take() { id.remove(); }
        let val = s.value();
        let p = proxy_c.clone();
        let a = apply.clone();
        let src = glib::timeout_add_local_once(Duration::from_millis(150), move || {
            a(&p, val);
        });
        *debounce.borrow_mut() = Some(src);
    });
}

pub fn connect_switch(
    switch: &gtk4::Switch,
    proxy: &Rc<SettingsProxy>,
    updating: &Rc<Cell<bool>>,
    apply: Rc<dyn Fn(&SettingsProxy, bool)>,
) {
    let proxy_c = proxy.clone();
    let updating_c = updating.clone();
    switch.connect_state_notify(move |sw| {
        if updating_c.get() { return; }
        apply(&proxy_c, sw.is_active());
    });
}

pub fn connect_entry(
    entry: &gtk4::Entry,
    proxy: &Rc<SettingsProxy>,
    updating: &Rc<Cell<bool>>,
    apply: Rc<dyn Fn(&SettingsProxy, &str)>,
) {
    let proxy_c = proxy.clone();
    let updating_c = updating.clone();
    let a = apply.clone();
    let apply_entry = entry.clone();
    entry.connect_activate(move |_| {
        if updating_c.get() { return; }
        a(&proxy_c, &apply_entry.text());
    });

    let proxy_c = proxy.clone();
    let updating_c = updating.clone();
    let a = apply;
    let focus_entry = entry.clone();
    let focus = gtk4::EventControllerFocus::new();
    focus.connect_leave(move |_| {
        if updating_c.get() { return; }
        a(&proxy_c, &focus_entry.text());
    });
    entry.add_controller(focus);
}
