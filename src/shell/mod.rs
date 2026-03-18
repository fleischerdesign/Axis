use std::rc::Rc;
use std::cell::RefCell;

pub trait ShellPopup {
    fn id(&self) -> &str;
    fn is_open(&self) -> bool;
    fn toggle(&self);
    fn close(&self);
}

pub struct ShellController {
    popups: RefCell<Vec<Rc<dyn ShellPopup>>>,
    bar_popup_state: Rc<RefCell<bool>>,
    active_id: Rc<RefCell<Option<String>>>,
    on_change: Box<dyn Fn()>,
}

impl ShellController {
    pub fn new(bar_popup_state: Rc<RefCell<bool>>, on_change: impl Fn() + 'static) -> Self {
        Self {
            popups: RefCell::new(Vec::new()),
            bar_popup_state,
            active_id: Rc::new(RefCell::new(None)),
            on_change: Box::new(on_change),
        }
    }

    pub fn add_popup(&self, popup: Rc<dyn ShellPopup>) {
        self.popups.borrow_mut().push(popup);
    }

    pub fn active_id(&self) -> Option<String> {
        self.active_id.borrow().clone()
    }

    pub fn toggle(&self, id: &str) {
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
        *self.bar_popup_state.borrow_mut() = any_open;
        (self.on_change)();
    }
}
