use crate::store::ReactiveBool;
use crate::widgets::base::PopupBase;
use gtk4::prelude::*;
use log::debug;
use std::cell::RefCell;
use std::rc::Rc;

pub trait PopupExt {
    fn id(&self) -> &str;
    fn base(&self) -> &PopupBase;

    fn on_open(&self) {}
    fn on_close(&self) {}

    fn is_open(&self) -> bool {
        self.base().is_open.get()
    }

    fn close(&self) {
        self.on_close();
        self.base().close();
    }

    fn open(&self) {
        self.on_open();
        self.base().open();
    }

    fn toggle(&self) {
        if self.is_open() {
            self.close();
        } else {
            self.open();
        }
    }

    fn handle_escape(&self) {
        self.close();
    }
}

pub struct ShellController {
    popups: RefCell<Vec<Rc<dyn PopupExt>>>,
    bar_popup_state: ReactiveBool,
    active_id: Rc<RefCell<Option<String>>>,
    on_change: Rc<dyn Fn()>,
}

impl ShellController {
    pub fn new(bar_popup_state: ReactiveBool, on_change: impl Fn() + 'static) -> Self {
        Self {
            popups: RefCell::new(Vec::new()),
            bar_popup_state,
            active_id: Rc::new(RefCell::new(None)),
            on_change: Rc::new(on_change),
        }
    }

    pub fn register<P: PopupExt + 'static>(&self, popup: &Rc<P>) {
        let on_change = self.on_change.clone();
        popup.base().window.connect_visible_notify(move |_| {
            on_change();
        });

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

        self.popups.borrow_mut().push(popup.clone());
    }

    pub fn active_id(&self) -> Option<String> {
        self.active_id.borrow().clone()
    }

    pub fn toggle(&self, id: &str) {
        debug!("[shell] Toggle: {id}");
        let popups = self.popups.borrow();
        let mut any_open_after = false;
        let mut new_id = None;

        for p in popups.iter() {
            if p.id() == id {
                p.toggle();
                if p.is_open() {
                    any_open_after = true;
                    new_id = Some(id.to_string());
                }
            } else if p.is_open() {
                p.close();
            }
        }

        *self.active_id.borrow_mut() = new_id;
        self.sync_state(any_open_after);
    }

    pub fn close_all(&self) {
        debug!("[shell] Close all");
        let popups = self.popups.borrow();
        for p in popups.iter() {
            if p.is_open() {
                p.close();
            }
        }
        *self.active_id.borrow_mut() = None;
        self.sync_state(false);
    }

    pub fn sync(&self) {
        let popups = self.popups.borrow();
        let mut any_open = false;
        let mut found_id = None;

        for p in popups.iter() {
            if p.is_open() {
                any_open = true;
                found_id = Some(p.id().to_string());
                break;
            }
        }

        *self.active_id.borrow_mut() = found_id;
        self.sync_state(any_open);
    }

    fn sync_state(&self, any_open: bool) {
        self.bar_popup_state.set(any_open);
        (self.on_change)();
    }
}
