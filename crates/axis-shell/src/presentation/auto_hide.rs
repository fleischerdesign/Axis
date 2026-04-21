use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, LayerShell};
use crate::utils::animations::SlideAnimator;

pub trait AutoHideView: IsA<gtk4::Window> + IsA<gtk4::Widget> + LayerShell + Clone + 'static {
    fn set_visible_state(&self, is_visible: bool);
}

pub struct AutoHidePresenter {
    is_visible: Rc<RefCell<bool>>,
    is_hovering: Rc<Cell<bool>>,
    current_generation: Rc<RefCell<u32>>,
    force_visible: Rc<Cell<bool>>,
    peek_px: i32,
    hide_delay_ms: u64,
}

impl AutoHidePresenter {
    pub fn new(peek_px: i32, hide_delay_ms: u64) -> Self {
        Self {
            is_visible: Rc::new(RefCell::new(false)),
            is_hovering: Rc::new(Cell::new(false)),
            current_generation: Rc::new(RefCell::new(0)),
            force_visible: Rc::new(Cell::new(false)),
            peek_px,
            hide_delay_ms,
        }
    }

    pub fn handle_enter<V: AutoHideView>(&self, view: &V) {
        self.is_hovering.set(true);
        let mut generation = self.current_generation.borrow_mut();
        *generation += 1;

        if !*self.is_visible.borrow() {
            *self.is_visible.borrow_mut() = true;
            SlideAnimator::slide_margin(
                view,
                Edge::Bottom,
                0,
                250,
            );
        }
    }

    pub fn handle_leave<V: AutoHideView>(&self, view: &V) {
        self.is_hovering.set(false);

        if self.force_visible.get() {
            return;
        }

        let is_visible_c = self.is_visible.clone();
        let view_c = view.clone();
        let bar_height = view.height();
        let target_margin = -(bar_height - self.peek_px);
        
        let mut gen_ref = self.current_generation.borrow_mut();
        *gen_ref += 1;
        let my_gen = *gen_ref;
        let gen_state = self.current_generation.clone();

        glib::timeout_add_local_once(Duration::from_millis(self.hide_delay_ms), move || {
            if *gen_state.borrow() == my_gen {
                *is_visible_c.borrow_mut() = false;
                SlideAnimator::slide_margin(
                    &view_c,
                    Edge::Bottom,
                    target_margin,
                    400,
                );
            }
        });
    }

    pub fn set_force_visible<V: AutoHideView>(&self, view: &V, visible: bool) {
        self.force_visible.set(visible);

        if visible {
            *self.current_generation.borrow_mut() += 1;

            if !*self.is_visible.borrow() {
                *self.is_visible.borrow_mut() = true;
                SlideAnimator::slide_margin(
                    view,
                    Edge::Bottom,
                    0,
                    250,
                );
            }
        } else if !self.is_hovering.get() {
            self.handle_leave(view);
        }
    }

    pub fn get_initial_margin(&self, actual_height: i32) -> i32 {
        -(actual_height - self.peek_px)
    }
}
