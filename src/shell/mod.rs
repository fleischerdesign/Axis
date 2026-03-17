use std::rc::Rc;
use std::cell::RefCell;

pub trait ShellPopup {
    fn toggle(&self);
    fn close(&self);
    fn is_open(&self) -> bool;
    fn id(&self) -> &str;
}

pub struct ShellController {
    popups: Vec<Rc<dyn ShellPopup>>,
    bar_popup_state: Rc<RefCell<bool>>,
    on_change: Box<dyn Fn() + 'static>,
}

impl ShellController {
    pub fn new(bar_popup_state: Rc<RefCell<bool>>, on_change: impl Fn() + 'static) -> Self {
        Self {
            popups: Vec::new(),
            bar_popup_state,
            on_change: Box::new(on_change),
        }
    }

    pub fn add_popup(&mut self, popup: Rc<dyn ShellPopup>) {
        self.popups.push(popup);
    }

    pub fn toggle(&self, id: &str) {
        let mut any_open = false;

        for popup in &self.popups {
            if popup.id() == id {
                popup.toggle();
            } else if popup.is_open() {
                popup.close();
            }

            if popup.is_open() {
                any_open = true;
            }
        }

        // Bar informieren
        *self.bar_popup_state.borrow_mut() = any_open;
        (self.on_change)();
    }

    pub fn close_all(&self) {
        for popup in &self.popups {
            if popup.is_open() {
                popup.close();
            }
        }
        *self.bar_popup_state.borrow_mut() = false;
        (self.on_change)();
    }

    pub fn sync(&self) {
        let any_open = self.popups.iter().any(|p| p.is_open());
        *self.bar_popup_state.borrow_mut() = any_open;
        (self.on_change)();
    }
}
