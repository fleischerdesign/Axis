use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use axis_core::services::settings::config::*;
use crate::continuity_proxy::ContinuityProxy;
use crate::proxy::SettingsProxy;

// ── Canvas Constants ────────────────────────────────────────────────────

const CANVAS_VIRTUAL_WIDTH: f64 = 500.0;
const CANVAS_VIRTUAL_HEIGHT: f64 = 350.0;

const LOCAL_DEVICE_WIDTH: f64 = 120.0;
const LOCAL_DEVICE_HEIGHT: f64 = 80.0;
const PEER_DEVICE_WIDTH: f64 = 100.0;
const PEER_DEVICE_HEIGHT: f64 = 65.0;
const DEVICE_GAP: f64 = 15.0;
const CORNER_RADIUS: f64 = 10.0;
const PEER_CORNER_RADIUS: f64 = 8.0;
const EDGE_SNAP_THRESHOLD: f64 = 60.0;
const LABEL_FONT_SIZE: f64 = 11.0;
const ICON_FONT_SIZE: f64 = 16.0;
const DRAG_OPACITY: f64 = 0.5;

const ASSUMED_SCREEN_WIDTH: f64 = 1920.0;
const ASSUMED_SCREEN_HEIGHT: f64 = 1080.0;

// ── Types ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct RemoteDevice {
    device_id: String,
    device_name: String,
    side: ArrangementSide,
    offset: i32,
    rect: CanvasRect,
}

#[derive(Clone, Copy)]
struct CanvasRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl CanvasRect {
    fn contains(self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

struct ArrangementState {
    remote: Option<RemoteDevice>,
    local_rect: CanvasRect,
    scale_x: f64,
    scale_y: f64,
    dragging: bool,
    drag_offset_x: f64,
    drag_offset_y: f64,
    drag_pos_x: f64,
    drag_pos_y: f64,
}

// ── Coordinate Conversion ───────────────────────────────────────────────

fn canvas_rect_from_arrangement(
    side: ArrangementSide,
    offset: i32,
    local: &CanvasRect,
    sx: f64,
    sy: f64,
) -> CanvasRect {
    let offset_f = offset as f64;
    match side {
        ArrangementSide::Right => CanvasRect {
            x: local.x + local.w + DEVICE_GAP,
            y: local.y + offset_f * sy,
            w: PEER_DEVICE_WIDTH,
            h: PEER_DEVICE_HEIGHT,
        },
        ArrangementSide::Left => CanvasRect {
            x: local.x - PEER_DEVICE_WIDTH - DEVICE_GAP,
            y: local.y + offset_f * sy,
            w: PEER_DEVICE_WIDTH,
            h: PEER_DEVICE_HEIGHT,
        },
        ArrangementSide::Top => CanvasRect {
            x: local.x + offset_f * sx,
            y: local.y - PEER_DEVICE_HEIGHT - DEVICE_GAP,
            w: PEER_DEVICE_WIDTH,
            h: PEER_DEVICE_HEIGHT,
        },
        ArrangementSide::Bottom => CanvasRect {
            x: local.x + offset_f * sx,
            y: local.y + local.h + DEVICE_GAP,
            w: PEER_DEVICE_WIDTH,
            h: PEER_DEVICE_HEIGHT,
        },
    }
}

fn nearest_side(px: f64, py: f64, local: &CanvasRect) -> ArrangementSide {
    let dist_left = (px - local.x).abs();
    let dist_right = (px - (local.x + local.w)).abs();
    let dist_top = (py - local.y).abs();
    let dist_bottom = (py - (local.y + local.h)).abs();

    let min_h = dist_left.min(dist_right);
    let min_v = dist_top.min(dist_bottom);

    if min_h < min_v {
        if dist_left < dist_right { ArrangementSide::Left } else { ArrangementSide::Right }
    } else if dist_top < dist_bottom {
        ArrangementSide::Top
    } else {
        ArrangementSide::Bottom
    }
}

fn offset_from_canvas_pos(
    side: ArrangementSide,
    canvas_x: f64,
    canvas_y: f64,
    local: &CanvasRect,
    sx: f64,
    sy: f64,
) -> i32 {
    let raw = match side {
        ArrangementSide::Right | ArrangementSide::Left => (canvas_y - local.y) / sy,
        ArrangementSide::Top | ArrangementSide::Bottom => (canvas_x - local.x) / sx,
    };
    raw.round() as i32
}

// ── Layout Computation ──────────────────────────────────────────────────

fn compute_local_rect(canvas_w: f64, canvas_h: f64) -> CanvasRect {
    CanvasRect {
        x: (canvas_w - LOCAL_DEVICE_WIDTH) / 2.0,
        y: (canvas_h - LOCAL_DEVICE_HEIGHT) / 2.0,
        w: LOCAL_DEVICE_WIDTH,
        h: LOCAL_DEVICE_HEIGHT,
    }
}

fn compute_scales(canvas_w: f64, canvas_h: f64) -> (f64, f64) {
    let sx = canvas_w / ASSUMED_SCREEN_WIDTH;
    let sy = canvas_h / ASSUMED_SCREEN_HEIGHT;
    (sx.max(0.1), sy.max(0.1))
}

/// Look up the arrangement for a peer from persisted config.
fn arrangement_for_peer(configs: &[PeerPersistedConfig], peer_id: &str) -> (ArrangementSide, i32) {
    if let Some(c) = configs.iter().find(|p| p.device_id == peer_id) {
        let side = c.arrangement_side;
        let offset = match side {
            ArrangementSide::Left | ArrangementSide::Right => c.arrangement_y,
            ArrangementSide::Top | ArrangementSide::Bottom => c.arrangement_x,
        };
        return (side, offset);
    }
    (ArrangementSide::Right, 0)
}

// ── Drag Handling ───────────────────────────────────────────────────────

fn apply_snap(px: f64, py: f64, state: &mut ArrangementState, proxy: &Rc<SettingsProxy>) {
    let Some(ref remote) = state.remote else { return; };
    let device_id = remote.device_id.clone();

    let local = state.local_rect;
    let sx = state.scale_x;
    let sy = state.scale_y;

    // Drop position (top-left corner of the peer)
    let peer_x = px - state.drag_offset_x;
    let peer_y = py - state.drag_offset_y;
    let peer_cx = peer_x + PEER_DEVICE_WIDTH / 2.0;
    let peer_cy = peer_y + PEER_DEVICE_HEIGHT / 2.0;

    // Determine which edge is nearest (like GNOME Display Settings)
    let local_cx = local.x + local.w / 2.0;
    let local_cy = local.y + local.h / 2.0;

    let dx = peer_cx - local_cx;
    let dy = peer_cy - local_cy;

    let side = if dx.abs() > dy.abs() {
        if dx > 0.0 { ArrangementSide::Right } else { ArrangementSide::Left }
    } else {
        if dy > 0.0 { ArrangementSide::Bottom } else { ArrangementSide::Top }
    };

    // Snap to nearest edge with fixed gap, preserve offset along the edge
    let snapped = match side {
        ArrangementSide::Right => CanvasRect {
            x: local.x + local.w + DEVICE_GAP,
            y: peer_y,
            w: PEER_DEVICE_WIDTH,
            h: PEER_DEVICE_HEIGHT,
        },
        ArrangementSide::Left => CanvasRect {
            x: local.x - PEER_DEVICE_WIDTH - DEVICE_GAP,
            y: peer_y,
            w: PEER_DEVICE_WIDTH,
            h: PEER_DEVICE_HEIGHT,
        },
        ArrangementSide::Top => CanvasRect {
            x: peer_x,
            y: local.y - PEER_DEVICE_HEIGHT - DEVICE_GAP,
            w: PEER_DEVICE_WIDTH,
            h: PEER_DEVICE_HEIGHT,
        },
        ArrangementSide::Bottom => CanvasRect {
            x: peer_x,
            y: local.y + local.h + DEVICE_GAP,
            w: PEER_DEVICE_WIDTH,
            h: PEER_DEVICE_HEIGHT,
        },
    };

    // Screen-pixel offset from the preserved position along the edge
    let offset = match side {
        ArrangementSide::Right | ArrangementSide::Left => {
            ((snapped.y - local.y) / sy).round() as i32
        }
        ArrangementSide::Top | ArrangementSide::Bottom => {
            ((snapped.x - local.x) / sx).round() as i32
        }
    };

    // Update state
    if let Some(ref mut remote) = state.remote {
        remote.side = side;
        remote.offset = offset;
        remote.rect = snapped;
    }

    // Persist
    let mut cfg = proxy.config().continuity;
    if let Some(persisted) = cfg.peer_configs.iter_mut().find(|p| p.device_id == device_id) {
        persisted.arrangement_side = side;
        match side {
            ArrangementSide::Left | ArrangementSide::Right => {
                persisted.arrangement_y = offset;
            }
            ArrangementSide::Top | ArrangementSide::Bottom => {
                persisted.arrangement_x = offset;
            }
        }
    }

    let p = proxy.clone();
    let c = cfg;
    gtk4::glib::spawn_future_local(async move {
        let _ = p.set_continuity(&c).await;
        p.update_cache_continuity(c);
    });
}

// ── Public Widget ───────────────────────────────────────────────────────

pub struct ArrangementGrid {
    drawing_area: gtk4::DrawingArea,
    state: Rc<RefCell<ArrangementState>>,
}

impl ArrangementGrid {
    pub fn new(proxy: &Rc<SettingsProxy>, continuity: Option<&Rc<ContinuityProxy>>) -> Self {
        let drawing_area = gtk4::DrawingArea::builder()
            .css_classes(["arrangement-grid"])
            .hexpand(true)
            .vexpand(true)
            .halign(gtk4::Align::Fill)
            .valign(gtk4::Align::Fill)
            .can_target(true)
            .build();

        drawing_area.set_content_width(CANVAS_VIRTUAL_WIDTH as i32);
        drawing_area.set_content_height(CANVAS_VIRTUAL_HEIGHT as i32);

        let local = compute_local_rect(CANVAS_VIRTUAL_WIDTH, CANVAS_VIRTUAL_HEIGHT);
        let (sx, sy) = compute_scales(CANVAS_VIRTUAL_WIDTH, CANVAS_VIRTUAL_HEIGHT);

        let state = Rc::new(RefCell::new(ArrangementState {
            remote: None,
            local_rect: local,
            scale_x: sx,
            scale_y: sy,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            drag_pos_x: 0.0,
            drag_pos_y: 0.0,
        }));

        Self::setup_draw_func(&drawing_area, &state);
        Self::setup_drag_handler(&drawing_area, &state, proxy);

        // Subscribe to continuity runtime — only show the connected peer
        if let Some(cp) = continuity {
            let state_c = state.clone();
            let da_c = drawing_area.clone();
            let cp_c = cp.clone();
            let proxy_c = proxy.clone();
            cp.on_change(move || {
                let cont_state = cp_c.state();
                let config = proxy_c.config();
                let s = state_c.borrow();
                let local = s.local_rect;
                let sx = s.scale_x;
                let sy = s.scale_y;
                drop(s);

                let remote = if let Some(ref conn) = cont_state.active_connection {
                    let (side, offset) = arrangement_for_peer(&config.continuity.peer_configs, &conn.peer_id);
                    Some(RemoteDevice {
                        device_id: conn.peer_id.clone(),
                        device_name: conn.peer_name.clone(),
                        side,
                        offset,
                        rect: canvas_rect_from_arrangement(side, offset, &local, sx, sy),
                    })
                } else {
                    None
                };

                let mut s = state_c.borrow_mut();
                s.remote = remote;
                drop(s);

                da_c.queue_draw();
            });
        }

        Self { drawing_area, state }
    }

    pub fn widget(&self) -> &gtk4::DrawingArea {
        &self.drawing_area
    }

    fn setup_draw_func(drawing_area: &gtk4::DrawingArea, state: &Rc<RefCell<ArrangementState>>) {
        let state_c = state.clone();
        drawing_area.set_draw_func(move |_area, cr, _w, _h| {
            let s = state_c.borrow();

            // ── Grid background ──────────────────────────────────────
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.04);
            let step = 25.0;
            let mut x = step;
            while x < CANVAS_VIRTUAL_WIDTH {
                cr.move_to(x, 0.0);
                cr.line_to(x, CANVAS_VIRTUAL_HEIGHT);
                x += step;
            }
            let mut y = step;
            while y < CANVAS_VIRTUAL_HEIGHT {
                cr.move_to(0.0, y);
                cr.line_to(CANVAS_VIRTUAL_WIDTH, y);
                y += step;
            }
            let _ = cr.stroke();

            // ── Local device ─────────────────────────────────────────
            let lr = s.local_rect;
            rounded_rect(cr, lr.x, lr.y, lr.w, lr.h, CORNER_RADIUS);
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.25);
            let _ = cr.fill();

            rounded_rect(cr, lr.x, lr.y, lr.w, lr.h, CORNER_RADIUS);
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.8);
            cr.set_line_width(2.0);
            let _ = cr.stroke();

            cr.set_source_rgba(0.208, 0.518, 0.894, 1.0);
            cr.set_font_size(ICON_FONT_SIZE);
            let ext = cr.text_extents("This Device").unwrap();
            cr.move_to(
                lr.x + (lr.w - ext.width()) / 2.0,
                lr.y + (lr.h - ext.height()) / 2.0 + ext.height(),
            );
            let _ = cr.show_text("This Device");

            // ── Remote device (connected peer) ───────────────────────
            if let Some(ref remote) = s.remote {
                if !s.dragging {
                    draw_remote_device(cr, remote, 1.0);
                }
            }

            // ── Ghost during drag ────────────────────────────────────
            if s.dragging {
                if let Some(ref remote) = s.remote {
                    let ghost = RemoteDevice {
                        rect: CanvasRect {
                            x: s.drag_pos_x - s.drag_offset_x,
                            y: s.drag_pos_y - s.drag_offset_y,
                            ..remote.rect
                        },
                        ..remote.clone()
                    };
                    draw_remote_device(cr, &ghost, DRAG_OPACITY);
                }
            }

            // ── Placeholder when disconnected ───────────────────────
            if s.remote.is_none() {
                cr.set_source_rgba(0.5, 0.5, 0.5, 0.4);
                cr.set_font_size(LABEL_FONT_SIZE);
                let msg = "Connect a device to arrange it";
                let ext = cr.text_extents(msg).unwrap();
                cr.move_to(
                    (CANVAS_VIRTUAL_WIDTH - ext.width()) / 2.0,
                    CANVAS_VIRTUAL_HEIGHT - 30.0,
                );
                let _ = cr.show_text(msg);
            }
        });

        fn draw_remote_device(cr: &gtk4::cairo::Context, device: &RemoteDevice, opacity: f64) {
            let r = device.rect;

            // Fill
            rounded_rect(cr, r.x, r.y, r.w, r.h, PEER_CORNER_RADIUS);
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.15 * opacity);
            let _ = cr.fill();

            // Border
            rounded_rect(cr, r.x, r.y, r.w, r.h, PEER_CORNER_RADIUS);
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.8 * opacity);
            cr.set_line_width(2.0);
            let _ = cr.stroke();

            // Label
            cr.set_source_rgba(0.208, 0.518, 0.894, 1.0 * opacity);
            cr.set_font_size(LABEL_FONT_SIZE);

            let max_w = r.w - 12.0;
            let label = truncate_text(cr, &device.device_name, max_w);
            let ext = cr.text_extents(&label).unwrap();
            cr.move_to(
                r.x + (r.w - ext.width()) / 2.0,
                r.y + (r.h + ext.height()) / 2.0,
            );
            let _ = cr.show_text(&label);
        }

        fn rounded_rect(cr: &gtk4::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
            let r = r.min(w / 2.0).min(h / 2.0);
            cr.new_sub_path();
            cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
            cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
            cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
            cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
            cr.close_path();
        }

        fn truncate_text(cr: &gtk4::cairo::Context, text: &str, max_width: f64) -> String {
            let ext = cr.text_extents(text).unwrap();
            if ext.width() <= max_width {
                return text.to_string();
            }
            let mut truncated = text.to_string();
            while truncated.len() > 3 {
                truncated.pop();
                let test = format!("{truncated}...");
                let ext = cr.text_extents(&test).unwrap();
                if ext.width() <= max_width {
                    return test;
                }
            }
            "...".to_string()
        }
    }

    fn setup_drag_handler(
        drawing_area: &gtk4::DrawingArea,
        state: &Rc<RefCell<ArrangementState>>,
        proxy: &Rc<SettingsProxy>,
    ) {
        // ── Click: start drag on press ───────────────────────────────
        let click = gtk4::GestureClick::new();
        click.set_button(1);

        let state_press = state.clone();
        click.connect_pressed(move |_gesture, _n_press, px, py| {
            let mut s = state_press.borrow_mut();
            if let Some(ref remote) = s.remote {
                if remote.rect.contains(px, py) {
                    let rx = remote.rect.x;
                    let ry = remote.rect.y;
                    s.dragging = true;
                    s.drag_offset_x = px - rx;
                    s.drag_offset_y = py - ry;
                    s.drag_pos_x = px;
                    s.drag_pos_y = py;
                }
            }
        });

        drawing_area.add_controller(click);

        // ── Motion: track position + finish on button release ────────
        let motion = gtk4::EventControllerMotion::new();

        let state_motion = state.clone();
        let da_motion = drawing_area.clone();
        let proxy_motion = proxy.clone();
        motion.connect_motion(move |ctrl, px, py| {
            let mut s = state_motion.borrow_mut();
            if s.dragging {
                // Check if the primary button is still held
                let still_pressed = ctrl
                    .current_event()
                    .and_then(|ev| ev.modifier_state().contains(gtk4::gdk::ModifierType::BUTTON1_MASK).then_some(true))
                    .unwrap_or(false);

                if still_pressed {
                    s.drag_pos_x = px;
                    s.drag_pos_y = py;
                    da_motion.queue_draw();
                } else {
                    // Button released — snap and finish
                    apply_snap(px, py, &mut s, &proxy_motion);
                    s.dragging = false;
                    da_motion.queue_draw();
                }
            }
        });

        let state_leave = state.clone();
        let da_leave = drawing_area.clone();
        let proxy_leave = proxy.clone();
        motion.connect_leave(move |_ctrl| {
            let mut s = state_leave.borrow_mut();
            if s.dragging {
                let px = s.drag_pos_x;
                let py = s.drag_pos_y;
                apply_snap(px, py, &mut s, &proxy_leave);
                s.dragging = false;
                da_leave.queue_draw();
            }
        });

        drawing_area.add_controller(motion);
    }
}
