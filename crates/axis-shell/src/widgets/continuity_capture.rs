use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Instant;
use crate::app_context::AppContext;
use axis_core::services::continuity::{ContinuityCmd, PeerArrangement, SharingMode, Side};
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
                    // In both Idle and Receiving mode, the edge window goes on the
                    // side where the peer is (per our arrangement). In Idle it starts
                    // sharing; in Receiving it triggers a switch back.
                    let config = data.active_peer_config();
                    let side = config.arrangement.side;
                    let is_receiving = data.sharing_mode == SharingMode::Receiving;

                    let w = ctrl_c.create_edge_window(
                        &app_c,
                        side,
                        is_receiving,
                        data.screen_width,
                        data.screen_height,
                        data.remote_screen,
                        &config.arrangement,
                    );
                    w.present();
                    info!("[continuity:edge] presenting edge window for {:?}, screen={}x{}, remote={:?}, mode={:?}",
                        side, data.screen_width, data.screen_height, data.remote_screen, data.sharing_mode);
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
        side: Side,
        is_receiving: bool,
        screen_w: i32,
        screen_h: i32,
        remote_screen: Option<(i32, i32)>,
        arrangement: &PeerArrangement,
    ) -> gtk4::Window {
        let gtk_edge = match side {
            Side::Left => Edge::Left,
            Side::Right => Edge::Right,
            Side::Top => Edge::Top,
            Side::Bottom => Edge::Bottom,
        };

        // Calculate overlap area: where should the edge window be?
        let is_horizontal = matches!(side, Side::Left | Side::Right);
        let local_len = if is_horizontal { screen_h } else { screen_w };
        let remote_len = remote_screen
            .map(|(rw, rh)| if is_horizontal { rh } else { rw })
            .unwrap_or(local_len);

        let (overlap_start, overlap_end) = arrangement
            .overlap_on_local(local_len, remote_len)
            .unwrap_or((0, local_len));
        let overlap_len = overlap_end - overlap_start;

        info!("[continuity:edge] overlap: {}..{} (len={}) on {:?} edge", overlap_start, overlap_end, overlap_len, side);

        let window = gtk4::Window::builder()
            .application(app)
            .title(format!("Continuity Edge {:?}", side))
            .can_focus(false)
            .resizable(false)
            .decorated(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);

        // Anchor to the target edge + one perpendicular edge, then use margin to position.
        // For Left/Right: anchor to the edge + Top, use margin_top for offset.
        // For Top/Bottom: anchor to the edge + Left, use margin_left for offset.
        match side {
            Side::Left => {
                window.set_anchor(Edge::Left, true);
                window.set_anchor(Edge::Top, true);
                window.set_margin(Edge::Top, overlap_start);
            }
            Side::Right => {
                window.set_anchor(Edge::Right, true);
                window.set_anchor(Edge::Top, true);
                window.set_margin(Edge::Top, overlap_start);
            }
            Side::Top => {
                window.set_anchor(Edge::Top, true);
                window.set_anchor(Edge::Left, true);
                window.set_margin(Edge::Left, overlap_start);
            }
            Side::Bottom => {
                window.set_anchor(Edge::Bottom, true);
                window.set_anchor(Edge::Left, true);
                window.set_margin(Edge::Left, overlap_start);
            }
        }

        let edge_widget = gtk4::DrawingArea::new();
        if is_horizontal {
            edge_widget.set_content_width(2);
            edge_widget.set_content_height(overlap_len);
        } else {
            edge_widget.set_content_width(overlap_len);
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
            info!("[continuity:edge] cursor ENTERED edge {:?} at x={:.0} y={:.0}", gtk_edge, x, y);
        });

        let ctrl_c2 = self.clone();
        let overlap_start_f = overlap_start as f64;
        motion.connect_motion(move |_ctrl, x, y| {
            let mut last = ctrl_c2.last_trigger.borrow_mut();
            if last.elapsed() < std::time::Duration::from_millis(500) {
                return;
            }
            *last = Instant::now();
            drop(last);

            // The widget-local position within the edge window.
            // Convert to absolute screen position along the edge.
            let edge_pos = if is_horizontal {
                overlap_start_f + y
            } else {
                overlap_start_f + x
            };

            let cmd = if is_receiving {
                ContinuityCmd::StopSharing(edge_pos)
            } else {
                ContinuityCmd::StartSharing(side, edge_pos)
            };
            info!("[continuity:edge] transition triggered via {:?}, edge_pos={:.0}", side, edge_pos);
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
