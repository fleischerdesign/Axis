use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::rc::Rc;
use std::cell::RefCell;
use crate::app_context::AppContext;
use crate::services::continuity::{ContinuityCmd, SharingMode, Side, protocol};
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
    left_edge: RefCell<Option<gtk4::Window>>,
    right_edge: RefCell<Option<gtk4::Window>>,
    overlay: RefCell<Option<gtk4::Window>>,
    pressure: RefCell<f64>,
    last_pos: RefCell<Option<(f64, f64)>>,
    entry_side: RefCell<Option<Side>>,
    escape_counter: RefCell<(u32, std::time::Instant)>,
}

impl ContinuityCaptureController {
    const PRESSURE_THRESHOLD: f64 = 50.0; // Reduced threshold for testing
    const ESCAPE_EXIT_COUNT: u32 = 4;
    const ESCAPE_EXIT_WINDOW: std::time::Duration = std::time::Duration::from_millis(1000);

    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Rc<Self> {
        let controller = Rc::new(Self {
            ctx: ctx.clone(),
            left_edge: RefCell::new(None),
            right_edge: RefCell::new(None),
            overlay: RefCell::new(None),
            pressure: RefCell::new(0.0),
            last_pos: RefCell::new(None),
            entry_side: RefCell::new(None),
            escape_counter: RefCell::new((0, std::time::Instant::now())),
        });

        let ctrl_c = controller.clone();
        let app_c = app.clone();
        ctx.continuity.store.subscribe(move |data| {
            let mut left = ctrl_c.left_edge.borrow_mut();
            let mut right = ctrl_c.right_edge.borrow_mut();
            let mut overlay = ctrl_c.overlay.borrow_mut();

            // Edge windows are active when connected and idle
            let show_edges = data.enabled && data.active_connection.is_some() && data.sharing_mode == SharingMode::Idle;
            
            if show_edges {
                if left.is_none() {
                    let w = ctrl_c.create_edge_window(&app_c, Edge::Left);
                    w.present();
                    *left = Some(w);
                }
                if right.is_none() {
                    let w = ctrl_c.create_edge_window(&app_c, Edge::Right);
                    w.present();
                    *right = Some(w);
                }
            } else {
                if let Some(w) = left.take() { w.close(); }
                if let Some(w) = right.take() { w.close(); }
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
                *ctrl_c.last_pos.borrow_mut() = None;
                *ctrl_c.escape_counter.borrow_mut() = (0, std::time::Instant::now());
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
        window.set_anchor(side, true);
        
        // Give GTK some initial dimensions to avoid size.height > 0 crashes
        window.set_default_size(2, 500);
        window.set_width_request(2);

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
            
            *self.entry_side.borrow_mut() = Some(cmd_side);
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

        // Explicit size for overlay too
        window.set_default_size(100, 100);

        window.set_cursor_from_name(Some("none"));
        window.add_css_class("continuity-overlay");
        
        let motion = gtk4::EventControllerMotion::new();
        let ctrl_m = self.clone();
        motion.connect_motion(move |_ctrl, x, y| {
            let mut last_pos = ctrl_m.last_pos.borrow_mut();
            
            if let Some((lx, ly)) = *last_pos {
                let dx = x - lx;
                let dy = y - ly;
                
                if dx.abs() > 0.1 || dy.abs() > 0.1 {
                    let msg = protocol::Message::CursorMove { dx, dy };
                    let _ = ctrl_m.ctx.continuity.tx.try_send(ContinuityCmd::SendInput(msg));
                }
            }
            
            *last_pos = Some((x, y));
        });
        window.add_controller(motion);

        let click = gtk4::GestureClick::new();
        let ctrl_cp = self.clone();
        click.connect_pressed(move |_ctrl, _n, _x, _y| {
            let button = match _ctrl.current_button() {
                1 => 272,
                3 => 273,
                2 => 274,
                _ => 0,
            };
            if button != 0 {
                let msg = protocol::Message::PointerButton { button, state: 1 };
                let _ = ctrl_cp.ctx.continuity.tx.try_send(ContinuityCmd::SendInput(msg));
            }
        });
        let ctrl_cr = self.clone();
        click.connect_released(move |_ctrl, _n, _x, _y| {
            let button = match _ctrl.current_button() {
                1 => 272,
                3 => 273,
                2 => 274,
                _ => 0,
            };
            if button != 0 {
                let msg = protocol::Message::PointerButton { button, state: 0 };
                let _ = ctrl_cr.ctx.continuity.tx.try_send(ContinuityCmd::SendInput(msg));
            }
        });
        window.add_controller(click);

        let key = gtk4::EventControllerKey::new();
        let ctrl_k = self.clone();
        key.connect_key_pressed(move |_ctrl, keyval, keycode, _state| {
            if keyval == gtk4::gdk::Key::Escape {
                let mut esc = ctrl_k.escape_counter.borrow_mut();
                let now = std::time::Instant::now();
                if now.duration_since(esc.1) < Self::ESCAPE_EXIT_WINDOW {
                    esc.0 += 1;
                } else {
                    esc.0 = 1;
                }
                esc.1 = now;
                
                if esc.0 >= Self::ESCAPE_EXIT_COUNT {
                    info!("[continuity] emergency exit triggered");
                    let _ = ctrl_k.ctx.continuity.tx.try_send(ContinuityCmd::StopSharing);
                    return gtk4::glib::Propagation::Stop;
                }
            }

            let msg = protocol::Message::KeyPress { key: keycode - 8, state: 1 };
            let _ = ctrl_k.ctx.continuity.tx.try_send(ContinuityCmd::SendInput(msg));
            gtk4::glib::Propagation::Stop
        });
        
        let ctrl_kr = self.clone();
        key.connect_key_released(move |_ctrl, _keyval, keycode, _state| {
            let msg = protocol::Message::KeyRelease { key: keycode - 8 };
            let _ = ctrl_kr.ctx.continuity.tx.try_send(ContinuityCmd::SendInput(msg));
        });
        window.add_controller(key);
        
        window
    }
}
