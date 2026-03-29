use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::rc::Rc;
use std::cell::RefCell;
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
}

impl ContinuityCaptureController {
    const PRESSURE_THRESHOLD: f64 = 50.0; 

    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Rc<Self> {
        let controller = Rc::new(Self {
            ctx: ctx.clone(),
            edge_window: RefCell::new(None),
            overlay: RefCell::new(None),
            pressure: RefCell::new(0.0),
        });

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
                    *edge = Some(w);
                }
            } else {
                if let Some(w) = edge.take() { w.close(); }
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
            .title(format!("Continuity Edge {:?}", side))
            .can_focus(false)
            .resizable(false)
            .decorated(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        
        match side {
            Edge::Left => window.set_anchor(Edge::Right, false),
            Edge::Right => window.set_anchor(Edge::Left, false),
            Edge::Top => window.set_anchor(Edge::Bottom, false),
            Edge::Bottom => window.set_anchor(Edge::Top, false),
            _ => {}
        }
        
        if side == Edge::Left || side == Edge::Right {
            window.set_default_size(2, 500);
            window.set_width_request(2);
        } else {
            window.set_default_size(500, 2);
            window.set_height_request(2);
        }

        window.add_css_class("continuity-edge-debug");
        
        let motion = gtk4::EventControllerMotion::new();
        let ctrl_enter = self.clone();
        motion.connect_enter(move |_, _x, _y| {
            ctrl_enter.check_transition(side, 20.0);
        });

        let ctrl_motion = self.clone();
        motion.connect_motion(move |_ctrl, _x, _y| {
            ctrl_motion.check_transition(side, 5.0);
        });
        
        let ctrl_leave = self.clone();
        motion.connect_leave(move |_| {
            *ctrl_leave.pressure.borrow_mut() = 0.0;
        });

        window.add_controller(motion);
        window
    }

    fn check_transition(&self, side: Edge, increment: f64) {
        let mut p = self.pressure.borrow_mut();
        *p += increment;
        
        if *p >= Self::PRESSURE_THRESHOLD {
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
