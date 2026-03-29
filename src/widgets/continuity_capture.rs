use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Instant;
use crate::app_context::AppContext;
use crate::services::continuity::{ContinuityCmd, SharingMode, Side};
use log::info;

pub struct ContinuityCaptureController {
    ctx: AppContext,
    edge_window: RefCell<Option<gtk4::Window>>,
    overlay: RefCell<Option<gtk4::Window>>,
    last_trigger: RefCell<Instant>,
}

impl ContinuityCaptureController {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Rc<Self> {
        let controller = Rc::new(Self {
            ctx: ctx.clone(),
            edge_window: RefCell::new(None),
            overlay: RefCell::new(None),
            last_trigger: RefCell::new(Instant::now()),
        });

        let ctrl_c = controller.clone();
        let app_c = app.clone();
        ctx.continuity.store.subscribe(move |data| {
            let mut edge = ctrl_c.edge_window.borrow_mut();
            let mut overlay = ctrl_c.overlay.borrow_mut();

            let show_edge = data.enabled
                && data.active_connection.is_some()
                && (data.sharing_mode == SharingMode::Idle
                    || data.sharing_mode == SharingMode::Receiving);

            if show_edge {
                if edge.is_none() {
                    let (side, is_receiving) = if data.sharing_mode == SharingMode::Receiving {
                        (data.receiving_entry_side.unwrap_or(data.preferred_edge), true)
                    } else {
                        (data.preferred_edge, false)
                    };

                    let edge_side = match side {
                        Side::Left => Edge::Left,
                        Side::Right => Edge::Right,
                        Side::Top => Edge::Top,
                        Side::Bottom => Edge::Bottom,
                    };

                    let w = ctrl_c.create_edge_window(&app_c, edge_side, data.screen_width, data.screen_height, side, is_receiving);
                    w.present();
                    info!("[continuity:edge] presenting edge window for {:?}, screen={}x{}, mode={:?}", edge_side, data.screen_width, data.screen_height, data.sharing_mode);
                    *edge = Some(w);
                }
            } else {
                if let Some(w) = edge.take() {
                    info!("[continuity:edge] closing edge window");
                    w.close();
                }
            }

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

    fn create_edge_window(
        self: &Rc<Self>,
        app: &libadwaita::Application,
        edge: Edge,
        screen_w: i32,
        screen_h: i32,
        side: Side,
        is_receiving: bool,
    ) -> gtk4::Window {
        let window = gtk4::Window::builder()
            .application(app)
            .title(format!("Continuity Edge {:?}", edge))
            .can_focus(false)
            .resizable(false)
            .decorated(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);

        match edge {
            Edge::Left => {
                window.set_anchor(Edge::Left, true);
                window.set_anchor(Edge::Top, true);
                window.set_anchor(Edge::Bottom, true);
            }
            Edge::Right => {
                window.set_anchor(Edge::Right, true);
                window.set_anchor(Edge::Top, true);
                window.set_anchor(Edge::Bottom, true);
            }
            Edge::Top => {
                window.set_anchor(Edge::Top, true);
                window.set_anchor(Edge::Left, true);
                window.set_anchor(Edge::Right, true);
            }
            Edge::Bottom => {
                window.set_anchor(Edge::Bottom, true);
                window.set_anchor(Edge::Left, true);
                window.set_anchor(Edge::Right, true);
            }
            _ => {}
        }

        let edge_widget = gtk4::DrawingArea::new();
        if edge == Edge::Left || edge == Edge::Right {
            edge_widget.set_content_width(2);
            edge_widget.set_content_height(screen_h);
        } else {
            edge_widget.set_content_width(screen_w);
            edge_widget.set_content_height(2);
        }
        edge_widget.add_css_class("continuity-edge-debug");
        edge_widget.set_draw_func(|_, cr, w, h| {
            cr.set_source_rgba(1.0, 0.0, 0.0, 0.3);
            cr.rectangle(0.0, 0.0, w as f64, h as f64);
            let _ = cr.fill();
        });
        window.set_child(Some(&edge_widget));

        let motion = gtk4::EventControllerMotion::new();
        motion.connect_enter(move |_, x, y| {
            info!("[continuity:edge] cursor ENTERED edge {:?} at x={:.0} y={:.0}", edge, x, y);
        });

        let ctrl_c2 = self.clone();
        motion.connect_motion(move |_ctrl, _x, _y| {
            let mut last = ctrl_c2.last_trigger.borrow_mut();
            if last.elapsed() < std::time::Duration::from_millis(500) {
                return;
            }
            *last = Instant::now();
            drop(last);

            let cmd = if is_receiving {
                ContinuityCmd::StopSharing
            } else {
                ContinuityCmd::StartSharing(side)
            };
            info!("[continuity:edge] transition triggered via {:?}", edge);
            let _ = ctrl_c2.ctx.continuity.tx.try_send(cmd);
        });

        edge_widget.add_controller(motion);
        window
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
        
        window
    }
}
