pub mod dbus;

use crate::widgets::base::PopupBase;
use gtk4::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

pub trait PopupExt {
    fn id(&self) -> &str;
    fn base(&self) -> &PopupBase;

    fn on_open(&self) {}
    fn on_close(&self) {}
    fn handle_escape(&self) { self.close(); }

    fn is_open(&self) -> bool { self.base().is_open.get() }
    fn open(&self) { self.on_open(); self.base().open(); }
    fn close(&self) { self.on_close(); self.base().close(); }
    fn toggle(&self) { if self.is_open() { self.close(); } else { self.open(); } }
}

pub struct ShellController {
    popups: RefCell<HashMap<String, Rc<dyn PopupExt>>>,
    bar_popup_open: axis_core::Store<bool>,
    on_change: Rc<dyn Fn()>,
}

impl ShellController {
    pub fn new(bar_popup_open: axis_core::Store<bool>, on_change: impl Fn() + 'static) -> Self {
        Self {
            popups: RefCell::new(HashMap::new()),
            bar_popup_open,
            on_change: Rc::new(on_change),
        }
    }

    pub fn register(&self, popup: &Rc<impl PopupExt + 'static>) {
        let id = popup.id().to_string();
        self.popups.borrow_mut().insert(id.clone(), popup.clone());

        Self::wire_visibility(popup);
        Self::wire_bar_sync(popup, &self.bar_popup_open, &self.on_change);
        Self::wire_escape(popup);
    }

    fn wire_visibility(popup: &Rc<impl PopupExt + 'static>) {
        popup.base().window.connect_visible_notify(move |win| {
            if win.is_visible() {
                // Focus handling would go here
            }
        });
    }

    fn wire_bar_sync(
        popup: &Rc<impl PopupExt + 'static>,
        bar_state: &axis_core::Store<bool>,
        on_change: &Rc<dyn Fn()>,
    ) {
        let bar_state = bar_state.clone();
        let on_change = on_change.clone();
        popup.base().is_open.subscribe(move |&is_open| {
            bar_state.set(is_open);
            on_change();
        });
    }

    fn wire_escape(popup: &Rc<impl PopupExt + 'static>) {
        let popup_ref = popup.clone();
        let key = gtk4::EventControllerKey::new();
        key.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                popup_ref.handle_escape();
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        popup.base().window.add_controller(key);
    }

    pub fn toggle(&self, id: &str) {
        if let Some(popup) = self.popups.borrow().get(id) {
            if !popup.is_open() {
                self.close_all();
                popup.open();
            } else {
                popup.close();
            }
        }
    }

    pub fn close_all(&self) {
        for popup in self.popups.borrow().values() {
            popup.close();
        }
    }
}
