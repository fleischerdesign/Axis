use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static SWIPE_ID: AtomicUsize = AtomicUsize::new(0);

/// Generic swipe-to-dismiss wrapper. Works with mouse drag and trackpad scroll.
/// Domain-agnostic — can wrap any widget in any context.
pub struct SwipeDismiss {
    pub container: gtk4::Box,
    is_dragging: Rc<Cell<bool>>,
}

impl SwipeDismiss {
    pub fn new(child: &impl IsA<gtk4::Widget>, on_dismiss: impl Fn() + 'static) -> Self {
        let id = SWIPE_ID.fetch_add(1, Ordering::Relaxed);
        let css_class = format!("swipe-dismiss-inner-{}", id);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        let inner = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        inner.add_css_class(&css_class);
        inner.append(child);
        container.append(&inner);

        // CSS provider scoped to this instance only
        let provider = gtk4::CssProvider::new();
        #[allow(deprecated)]
        {
            let ctx = inner.style_context();
            ctx.add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_USER);
        }

        let on_dismiss = Rc::new(on_dismiss);

        // --- State ---
        let is_swiped = Rc::new(Cell::new(false));
        let is_dragging = Rc::new(Cell::new(false));
        let acc_dx = Rc::new(Cell::new(0.0));
        let scroll_timeout: Rc<RefCell<Option<gtk4::glib::SourceId>>> = Rc::new(RefCell::new(None));

        // --- DRAG (mouse / touch) ---
        let drag = gtk4::GestureDrag::new();
        drag.set_propagation_phase(gtk4::PropagationPhase::Capture);

        let is_d = is_dragging.clone();
        drag.connect_drag_begin(move |_, _, _| {
            is_d.set(true);
        });

        let prov_d = provider.clone();
        let cls_d = css_class.clone();
        drag.connect_drag_update(move |_, offset_x, _| {
            let x = offset_x.clamp(-380.0, 380.0);
            let progress = (x.abs() / 150.0).clamp(0.0, 1.0);
            let opacity = 1.0 - progress;
            prov_d.load_from_string(&format!(
                ".{} {{ transform: translateX({}px); opacity: {:.2}; transition: none; }}",
                cls_d,
                x.round() as i32,
                opacity
            ));
        });

        let prov_de = provider.clone();
        let cls_de = css_class.clone();
        let is_de = is_dragging.clone();
        let is_sw = is_swiped.clone();
        let on_d = on_dismiss.clone();
        drag.connect_drag_end(move |_, offset_x, _| {
            is_de.set(false);

            if offset_x.abs() > 100.0 {
                is_sw.set(true);
                let ws = is_sw.clone();
                gtk4::glib::timeout_add_local_once(
                    std::time::Duration::from_millis(100),
                    move || ws.set(false),
                );
                on_d();
            } else {
                prov_de.load_from_string(&format!(
                    ".{} {{ transform: translateX(0px); opacity: 1.0; transition: all 0.2s ease-out; }}",
                    cls_de
                ));
            }
        });
        container.add_controller(drag);

        // --- SCROLL (trackpad 2-finger) ---
        let scroll = gtk4::EventControllerScroll::new(
            gtk4::EventControllerScrollFlags::HORIZONTAL
                | gtk4::EventControllerScrollFlags::KINETIC,
        );
        scroll.set_propagation_phase(gtk4::PropagationPhase::Capture);

        let is_ds = is_dragging.clone();
        let acc_s = acc_dx.clone();
        let prov_s = provider.clone();
        let cls_s = css_class.clone();
        let tout_s = scroll_timeout.clone();
        let on_s = on_dismiss.clone();

        scroll.connect_scroll(move |_, dx, _| {
            if is_ds.get() {
                return gtk4::glib::Propagation::Stop;
            }

            let x = (acc_s.get() - dx * 3.0).clamp(-380.0, 380.0);
            acc_s.set(x);

            let progress = (x.abs() / 150.0).clamp(0.0, 1.0);
            let opacity = 1.0 - progress;
            prov_s.load_from_string(&format!(
                ".{} {{ transform: translateX({}px); opacity: {:.2}; transition: none; }}",
                cls_s,
                x.round() as i32,
                opacity
            ));

            if x.abs() > 120.0 {
                on_s();
                return gtk4::glib::Propagation::Stop;
            }

            // Debounce reset
            if let Some(src) = tout_s.borrow_mut().take() {
                src.remove();
            }

            let acc_r = acc_s.clone();
            let prov_r = prov_s.clone();
            let cls_r = cls_s.clone();
            let tout_r = tout_s.clone();
            let src = gtk4::glib::timeout_add_local_once(
                std::time::Duration::from_millis(200),
                move || {
                    prov_r.load_from_string(&format!(
                        ".{} {{ transform: translateX(0px); opacity: 1.0; transition: all 0.3s ease-out; }}",
                        cls_r
                    ));
                    acc_r.set(0.0);
                    *tout_r.borrow_mut() = None;
                },
            );
            *tout_s.borrow_mut() = Some(src);

            gtk4::glib::Propagation::Stop
        });

        let prov_se = provider.clone();
        let cls_se = css_class.clone();
        scroll.connect_scroll_end(move |_| {
            prov_se.load_from_string(&format!(
                ".{} {{ transform: translateX(0px); opacity: 1.0; transition: all 0.3s ease-out; }}",
                cls_se
            ));
            acc_dx.set(0.0);
        });

        container.add_controller(scroll);

        // Cleanup on destroy
        let st = scroll_timeout;
        container.connect_destroy(move |_| {
            if let Some(src) = st.borrow_mut().take() {
                src.remove();
            }
        });

        Self {
            container,
            is_dragging,
        }
    }

    /// Returns whether a drag gesture is currently active.
    /// Use this to guard click handlers against false triggers during drags.
    pub fn is_dragging(&self) -> bool {
        self.is_dragging.get()
    }
}
