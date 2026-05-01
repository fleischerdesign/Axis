use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell, KeyboardMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use axis_domain::models::continuity::{ContinuityStatus, PeerArrangement, SharingState, Side};
use axis_domain::ports::continuity::ContinuityProvider;
use axis_presentation::View;

pub struct ContinuityCaptureController {
    provider: Arc<dyn ContinuityProvider>,
    app: libadwaita::Application,
    edge_window: Rc<RefCell<Option<gtk4::Window>>>,
    overlay: Rc<RefCell<Option<gtk4::Window>>>,
    last_trigger: Rc<RefCell<Instant>>,
    last_arrangement: Rc<RefCell<PeerArrangement>>,
    is_receiving: Rc<RefCell<bool>>,
}

impl ContinuityCaptureController {
    pub fn new(
        app: &libadwaita::Application,
        provider: Arc<dyn ContinuityProvider>,
    ) -> Self {
        Self {
            provider,
            app: app.clone(),
            edge_window: Rc::new(RefCell::new(None)),
            overlay: Rc::new(RefCell::new(None)),
            last_trigger: Rc::new(RefCell::new(Instant::now())),
            last_arrangement: Rc::new(RefCell::new(PeerArrangement { side: Side::Right, offset: 0 })),
            is_receiving: Rc::new(RefCell::new(false)),
        }
    }

    fn create_edge_window(
        &self,
        side: Side,
        is_receiving: bool,
        screen_w: i32,
        screen_h: i32,
        remote_screen: Option<(i32, i32)>,
        arrangement: &PeerArrangement,
    ) -> gtk4::Window {
        let is_horizontal = matches!(side, Side::Left | Side::Right);
        let local_len = if is_horizontal { screen_h } else { screen_w };
        let remote_len = remote_screen
            .map(|(rw, rh)| if is_horizontal { rh } else { rw })
            .unwrap_or(local_len);

        let (overlap_start, overlap_end) = arrangement
            .overlap_on_local(local_len, remote_len)
            .unwrap_or((0, local_len));
        let overlap_len = overlap_end - overlap_start;

        log::info!(
            "[continuity:edge] overlap: {}..{} (len={}) on {:?} edge",
            overlap_start, overlap_end, overlap_len, side
        );

        let window = gtk4::Window::builder()
            .application(&self.app)
            .title(format!("Continuity Edge {:?}", side))
            .can_focus(false)
            .resizable(false)
            .decorated(false)
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);

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
        let provider = self.provider.clone();
        let last_trigger = self.last_trigger.clone();
        let overlap_start_f = overlap_start as f64;
        motion.connect_motion(move |_ctrl, x, y| {
            let mut last = last_trigger.borrow_mut();
            if last.elapsed() < std::time::Duration::from_millis(50) {
                return;
            }
            *last = Instant::now();
            drop(last);

            let edge_pos = if is_horizontal {
                overlap_start_f + y
            } else {
                overlap_start_f + x
            };

            let p = provider.clone();
            if is_receiving {
                log::info!("[continuity:edge] stop_sharing via {:?}, edge_pos={:.0}", side, edge_pos);
                tokio::spawn(async move {
                    let _ = p.stop_sharing(edge_pos).await;
                });
            } else {
                log::info!("[continuity:edge] start_sharing via {:?}, edge_pos={:.0}", side, edge_pos);
                tokio::spawn(async move {
                    let _ = p.start_sharing(side, edge_pos).await;
                });
            }
        });

        motion.connect_enter(|_, x, y| {
            log::info!("[continuity:edge] cursor ENTERED at x={:.0} y={:.0}", x, y);
        });

        edge_widget.add_controller(motion);
        window
    }

    fn create_capture_overlay(&self) -> gtk4::Window {
        let window = gtk4::Window::builder()
            .application(&self.app)
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

impl View<ContinuityStatus> for ContinuityCaptureController {
    fn render(&self, data: &ContinuityStatus) {
        let mut edge = self.edge_window.borrow_mut();
        let mut overlay = self.overlay.borrow_mut();

        let show_edge = data.enabled
            && data.active_connection.is_some()
            && (data.sharing_state.is_idle() || matches!(data.sharing_state, SharingState::Receiving));

        if show_edge {
            let config = data.active_peer_config();
            let side = config.arrangement.side;
            let receiving = matches!(data.sharing_state, SharingState::Receiving);

            let arrangement_changed = edge.is_some()
                && *self.last_arrangement.borrow() != config.arrangement;

            if edge.is_none() || arrangement_changed {
                if let Some(w) = edge.take() {
                    log::info!("[continuity:edge] closing edge window for repositioning");
                    w.close();
                }

                let w = self.create_edge_window(
                    side,
                    receiving,
                    data.screen_width,
                    data.screen_height,
                    data.remote_screen,
                    &config.arrangement,
                );
                w.present();
                log::info!(
                    "[continuity:edge] presenting edge window for {:?}, screen={}x{}, remote={:?}",
                    side, data.screen_width, data.screen_height, data.remote_screen
                );
                *edge = Some(w);
                *self.last_arrangement.borrow_mut() = config.arrangement;
            }

            *self.is_receiving.borrow_mut() = receiving;
        } else {
            if let Some(w) = edge.take() {
                log::info!("[continuity:edge] closing edge window");
                w.close();
            }
        }

        let show_overlay = data.enabled
            && matches!(data.sharing_state, SharingState::Sharing { .. });
        if show_overlay {
            if overlay.is_none() {
                let w = self.create_capture_overlay();
                w.present();
                *overlay = Some(w);
            }
        } else {
            if let Some(w) = overlay.take() {
                w.close();
            }
        }
    }
}
