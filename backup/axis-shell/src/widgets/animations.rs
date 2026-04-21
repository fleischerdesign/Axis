use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub struct SlideAnimator;

impl SlideAnimator {
    pub const INTERVAL_MS: u64 = 16;
    pub const STEP_PX: i32 = 8;

    /// Animiert den Margin eines Widgets an einer bestimmten Kante (LayerShell-spezifisch)
    pub fn slide_margin<W: IsA<gtk4::Window> + LayerShell + Clone + 'static>(
        window: &W,
        edge: Edge,
        target: i32,
        active_anim: Rc<RefCell<Option<glib::SourceId>>>,
    ) {
        // Bestehende Animation für dieses Widget abbrechen
        if let Some(src) = active_anim.borrow_mut().take() {
            src.remove();
        }

        let window_c = window.clone();
        let anim_c = active_anim.clone();

        let src = glib::timeout_add_local(Duration::from_millis(Self::INTERVAL_MS), move || {
            let current = window_c.margin(edge);

            if current == target {
                *anim_c.borrow_mut() = None;
                return glib::ControlFlow::Break;
            }

            let next = if current < target {
                (current + Self::STEP_PX).min(target)
            } else {
                (current - Self::STEP_PX).max(target)
            };

            window_c.set_margin(edge, next);

            if next == target {
                *anim_c.borrow_mut() = None;
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        *active_anim.borrow_mut() = Some(src);
    }
}
