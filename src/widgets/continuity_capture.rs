use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Instant;
use crate::app_context::AppContext;
use crate::services::continuity::{ContinuityCmd, SharingMode, Side};
use log::info;

/*
 * NOTE: This implementation uses a fullscreen transparent overlay to capture input
 * because Wayland does not allow global input hooking for security reasons.
 *
 * TODO: Switch to ext-virtual-input-v1 or a specialized Niri IPC protocol 
 * once they are mature and supported by our target environments to avoid 
 * the "invisible window hack".
 */

pub struct ContinuityCaptureController {
    ctx: AppContext,
    edge_window: RefCell<Option<gtk4::Window>>,
    overlay: RefCell<Option<gtk4::Window>>,
    pressure: RefCell<f64>,
    last_trigger: RefCell<Instant>,
}

impl ContinuityCaptureController {
    const PRESSURE_THRESHOLD: f64 = 50.0; 

    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Rc<Self> {
        let controller = Rc::new(Self {
            ctx: ctx.clone(),
            edge_window: RefCell::new(None),
            overlay: RefCell::new(None),
            pressure: RefCell::new(0.0),
            last_trigger: RefCell::new(Instant::now()),
        });

        // Screen size is detected by the continuity service from Niri outputs.
        // The capture controller does NOT re-query to avoid picking a different
        // monitor on multi-monitor setups (e.g. 2560x1440 vs 1920x1080).

        let ctrl_c = controller.clone();
        let app_c = app.clone();
        ctx.continuity.store.subscribe(move |data| {
            let mut edge = ctrl_c.edge_window.borrow_mut();
            let mut overlay = ctrl_c.overlay.borrow_mut();

            // Edge windows are active when connected and idle
            let show_edges = data.enabled && data.active_connection.is_some() && data.sharing_mode == SharingMode::Idle;
            
            if show_edges {
                if edge.is_none() {
                    let side = match data.preferred_edge {
                        Side::Left => Edge::Left,
                        Side::Right => Edge::Right,
                        Side::Top => Edge::Top,
                        Side::Bottom => Edge::Bottom,
                    };
                    let w = ctrl_c.create_edge_window(&app_c, side);
                    w.present();
                    info!("[continuity:edge] presenting edge window for {:?}", side);
                    *edge = Some(w);
                }
            } else {
                if let Some(w) = edge.take() {
                    info!("[continuity:edge] closing edge window");
                    w.close();
                }
                *ctrl_c.pressure.borrow_mut() = 0.0;
            }

            // Overlay is active when sharing
            let show_overlay = data.enabled && data.sharing_mode == SharingMode::Sharing;
            if show_overlay {
                if overlay.is_none() {
                    let w = ctrl_c.create_capture_overlay(&app_c);
                    w.present();
                    *overlay = Some(w);
                }
            } else {
                if let Some(w) = overlay.take() { w.close(); }
            }
        });

        controller
    }

    fn create_edge_window(self: &Rc<Self>, app: &libadwaita::Application, side: Edge) -> gtk4::Window {
        let window = gtk4::Window::builder()
            .application(app)
            .title("Continuity Edge")
            .can_focus(false)
            .resizable(true)
            .decorated(false)
            .build();

        window.fullscreen();
        window.set_opacity(0.01);
        window.set_cursor_from_name(Some("none"));

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        window.set_child(Some(&container));

        // Log window dimensions once realized
        let side_dbg = side;
        window.connect_map(move |w| {
            info!("[continuity:edge] window mapped, side={:?}, allocated={}x{}", side_dbg, w.allocated_width(), w.allocated_height());
        });

        let motion = gtk4::EventControllerMotion::new();
        let ctrl_c = self.clone();
        motion.connect_motion(move |ctrl, x, y| {
            let (width, height) = if let Some(w) = ctrl.widget().and_downcast::<gtk4::Window>() {
                (w.allocated_width() as f64, w.allocated_height() as f64)
            } else {
                (0.0, 0.0)
            };

            let at_edge = match side {
                Edge::Left => x <= 2.0,
                Edge::Right => width > 0.0 && x >= width - 2.0,
                Edge::Top => y <= 2.0,
                Edge::Bottom => height > 0.0 && y >= height - 2.0,
                _ => false,
            };

            // Log every 50th motion event to avoid spam
            if (x as i64) % 200 < 5 {
                info!("[continuity:edge] motion {:?}: x={:.0} y={:.0} size={}x{} at_edge={} pressure={:.0}", side, x, y, width, height, at_edge, *ctrl_c.pressure.borrow());
            }

            if at_edge {
                ctrl_c.check_transition(side, 5.0);
            }
        });

        let ctrl_leave = self.clone();
        motion.connect_leave(move |_| {
            info!("[continuity:edge] cursor left edge window {:?}", side);
            *ctrl_leave.pressure.borrow_mut() = 0.0;
        });

        container.add_controller(motion);
        window
    }

    fn check_transition(&self, side: Edge, increment: f64) {
        let mut p = self.pressure.borrow_mut();
        *p += increment;
        
        if *p >= Self::PRESSURE_THRESHOLD {
            let mut last = self.last_trigger.borrow_mut();
            if last.elapsed() < std::time::Duration::from_secs(1) {
                return;
            }
            *last = std::time::Instant::now();

            info!("[continuity] transition triggered via {:?}", side);
            *p = 0.0;
            
            let cmd_side = match side {
                Edge::Left => Side::Left,
                Edge::Right => Side::Right,
                Edge::Top => Side::Top,
                Edge::Bottom => Side::Bottom,
                _ => Side::Left,
            };
            
            let _ = self.ctx.continuity.tx.try_send(ContinuityCmd::StartSharing(cmd_side));
        }
    }

    fn create_capture_overlay(self: &Rc<Self>, app: &libadwaita::Application) -> gtk4::Window {
        let window = gtk4::Window::builder()
            .application(app)
            .title("Continuity Capture Overlay")
            .can_focus(true)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_exclusive_zone(-1); 
        window.set_keyboard_mode(KeyboardMode::Exclusive);

        window.set_default_size(100, 100);
        window.set_cursor_from_name(Some("none"));
        window.add_css_class("continuity-overlay");
        
        // We no longer need EventControllers here as evdev takes over.
        // We only need the window to hide the cursor and signal intent to Niri.
        
        window
    }
}
