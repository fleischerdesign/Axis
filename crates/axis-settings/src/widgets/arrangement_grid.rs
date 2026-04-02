use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use axis_core::services::continuity::{PeerArrangement, Side};
use axis_core::services::settings::config::*;
use crate::continuity_proxy::ContinuityProxy;
use crate::proxy::SettingsProxy;

// ── Canvas Constants ────────────────────────────────────────────────────

const CANVAS_VIRTUAL_WIDTH: f64 = 500.0;
const CANVAS_VIRTUAL_HEIGHT: f64 = 350.0;

const CANVAS_GAP: f64 = 15.0;
const CORNER_RADIUS: f64 = 10.0;
const PEER_CORNER_RADIUS: f64 = 8.0;
const LABEL_FONT_SIZE: f64 = 11.0;
const ICON_FONT_SIZE: f64 = 16.0;
const DRAG_OPACITY: f64 = 0.5;

/// Padding around the combined layout inside the canvas.
const CANVAS_PADDING: f64 = 30.0;

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
    peer_rect: CanvasRect,
    scale: f64,
    dragging: bool,
    drag_offset_x: f64,
    drag_offset_y: f64,
    drag_pos_x: f64,
    drag_pos_y: f64,
}

// ── Layout Computation ──────────────────────────────────────────────────

/// Compute a uniform scale so that both screens (with a gap between them)
/// fit inside the canvas, preserving the real aspect ratio.
///
/// Returns `(local_rect, scale, peer_rect_at_origin)` where `peer_rect_at_origin`
/// is the peer rect snapped to `Right` with offset 0.
fn compute_layout(
    canvas_w: f64,
    canvas_h: f64,
    local_w: i32,
    local_h: i32,
    peer_w: i32,
    peer_h: i32,
) -> (CanvasRect, f64, CanvasRect) {
    let lw = local_w as f64;
    let lh = local_h as f64;
    let pw = peer_w as f64;
    let ph = peer_h as f64;

    // Canvas area available for layout (both rects + gap must fit)
    let avail_w = canvas_w - 2.0 * CANVAS_PADDING;
    let avail_h = canvas_h - 2.0 * CANVAS_PADDING;

    // Worst-case: screens placed side-by-side horizontally or vertically
    let total_w = lw + CANVAS_GAP + pw;
    let total_h = lh.max(ph);

    let scale = (avail_w / total_w).min(avail_h / total_h).max(0.01);

    let slw = lw * scale;
    let slh = lh * scale;
    let spw = pw * scale;
    let sph = ph * scale;

    // Centre the combined layout in the canvas
    let combined_w = slw + CANVAS_GAP + spw;
    let combined_h = slh.max(sph);
    let origin_x = (canvas_w - combined_w) / 2.0;
    let origin_y = (canvas_h - combined_h) / 2.0;

    let local_rect = CanvasRect {
        x: origin_x,
        y: origin_y + (combined_h - slh) / 2.0,
        w: slw,
        h: slh,
    };

    // Peer at Right, offset 0: sits directly to the right of local, top-aligned
    let peer_rect = CanvasRect {
        x: local_rect.x + slw + CANVAS_GAP,
        y: local_rect.y,
        w: spw,
        h: sph,
    };

    (local_rect, scale, peer_rect)
}

// ── Coordinate Conversion ───────────────────────────────────────────────

/// Convert an arrangement (side + screen-pixel offset) into a canvas rect.
fn canvas_rect_from_arrangement(
    side: ArrangementSide,
    offset: i32,
    local: &CanvasRect,
    peer_w: f64,
    peer_h: f64,
    scale: f64,
) -> CanvasRect {
    let offset_canvas = offset as f64 * scale;
    match side {
        ArrangementSide::Right => CanvasRect {
            x: local.x + local.w + CANVAS_GAP,
            y: local.y + offset_canvas,
            w: peer_w,
            h: peer_h,
        },
        ArrangementSide::Left => CanvasRect {
            x: local.x - peer_w - CANVAS_GAP,
            y: local.y + offset_canvas,
            w: peer_w,
            h: peer_h,
        },
        ArrangementSide::Top => CanvasRect {
            x: local.x + offset_canvas,
            y: local.y - peer_h - CANVAS_GAP,
            w: peer_w,
            h: peer_h,
        },
        ArrangementSide::Bottom => CanvasRect {
            x: local.x + offset_canvas,
            y: local.y + local.h + CANVAS_GAP,
            w: peer_w,
            h: peer_h,
        },
    }
}

/// Convert a Side to ArrangementSide for storage.
fn side_to_arrangement(s: Side) -> ArrangementSide {
    match s {
        Side::Left => ArrangementSide::Left,
        Side::Right => ArrangementSide::Right,
        Side::Top => ArrangementSide::Top,
        Side::Bottom => ArrangementSide::Bottom,
    }
}

/// Convert an ArrangementSide to Side for the continuity service.
fn side_to_continuity(s: ArrangementSide) -> Side {
    match s {
        ArrangementSide::Left => Side::Left,
        ArrangementSide::Right => Side::Right,
        ArrangementSide::Top => Side::Top,
        ArrangementSide::Bottom => Side::Bottom,
    }
}

// ── Drag Handling ───────────────────────────────────────────────────────

fn apply_snap(
    px: f64,
    py: f64,
    state: &mut ArrangementState,
    proxy: &Rc<SettingsProxy>,
    continuity: Option<&Rc<ContinuityProxy>>,
) {
    let Some(ref remote) = state.remote else { return; };
    let device_id = remote.device_id.clone();

    let local = state.local_rect;
    let scale = state.scale;
    let pw = remote.rect.w;
    let ph = remote.rect.h;

    // Drop position (top-left corner of the peer)
    let peer_x = px - state.drag_offset_x;
    let peer_y = py - state.drag_offset_y;
    let peer_cx = peer_x + pw / 2.0;
    let peer_cy = peer_y + ph / 2.0;

    let local_cx = local.x + local.w / 2.0;
    let local_cy = local.y + local.h / 2.0;

    let dx = peer_cx - local_cx;
    let dy = peer_cy - local_cy;

    let side = if dx.abs() > dy.abs() {
        if dx > 0.0 { ArrangementSide::Right } else { ArrangementSide::Left }
    } else {
        if dy > 0.0 { ArrangementSide::Bottom } else { ArrangementSide::Top }
    };

    // Snap peer rect to the chosen edge, preserving the dragged offset along it
    let snapped = canvas_rect_from_arrangement(side, 0, &local, pw, ph, scale);
    let snapped = match side {
        ArrangementSide::Right | ArrangementSide::Left => CanvasRect { y: peer_y, ..snapped },
        ArrangementSide::Top | ArrangementSide::Bottom => CanvasRect { x: peer_x, ..snapped },
    };

    // Screen-pixel offset (inverse of canvas_rect_from_arrangement)
    let offset = match side {
        ArrangementSide::Right | ArrangementSide::Left => {
            ((snapped.y - local.y) / scale).round() as i32
        }
        ArrangementSide::Top | ArrangementSide::Bottom => {
            ((snapped.x - local.x) / scale).round() as i32
        }
    };

    // Update state
    if let Some(ref mut remote) = state.remote {
        remote.side = side;
        remote.offset = offset;
        remote.rect = snapped;
    }

    // Persist to settings
    persist_arrangement(proxy, &device_id, side, offset);

    // Notify Continuity Service
    if let Some(cp) = continuity {
        let arr = PeerArrangement { side: side_to_continuity(side), offset };
        let cp = cp.clone();
        gtk4::glib::spawn_future_local(async move {
            let _ = cp.set_peer_arrangement(&arr).await;
        });
    }
}

fn persist_arrangement(
    proxy: &Rc<SettingsProxy>,
    device_id: &str,
    side: ArrangementSide,
    offset: i32,
) {
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
    gtk4::glib::spawn_future_local(async move {
        let _ = p.set_continuity(&cfg).await;
        p.update_cache_continuity(cfg);
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

        // Default layout — will be replaced once the continuity service reports
        // the real screen dimensions.
        let (local_rect, scale, peer_rect) = compute_layout(
            CANVAS_VIRTUAL_WIDTH, CANVAS_VIRTUAL_HEIGHT,
            1920, 1080, 1920, 1080,
        );

        let state = Rc::new(RefCell::new(ArrangementState {
            remote: None,
            local_rect,
            peer_rect,
            scale,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            drag_pos_x: 0.0,
            drag_pos_y: 0.0,
        }));

        Self::setup_draw_func(&drawing_area, &state);
        Self::setup_drag_handler(&drawing_area, &state, proxy, continuity.cloned());

        // Subscribe to continuity runtime — show the connected peer
        if let Some(cp) = continuity {
            let state_c = state.clone();
            let da_c = drawing_area.clone();
            let cp_c = cp.clone();
            cp.on_change(move || {
                let cont_state = cp_c.state();
                let s = state_c.borrow();
                let local = s.local_rect;
                let scale = s.scale;
                let pw = s.peer_rect.w;
                let ph = s.peer_rect.h;
                drop(s);

                let remote = if let Some(ref conn) = cont_state.active_connection {
                    let pc = cont_state.peer_configs.get(&conn.peer_id);
                    let arrangement = pc.map(|p| p.arrangement).unwrap_or_default();
                    let side = side_to_arrangement(arrangement.side);
                    let offset = arrangement.offset;

                    Some(RemoteDevice {
                        device_id: conn.peer_id.clone(),
                        device_name: conn.peer_name.clone(),
                        side,
                        offset,
                        rect: canvas_rect_from_arrangement(side, offset, &local, pw, ph, scale),
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

    // ── Drawing ─────────────────────────────────────────────────────

    fn setup_draw_func(drawing_area: &gtk4::DrawingArea, state: &Rc<RefCell<ArrangementState>>) {
        let state_c = state.clone();
        drawing_area.set_draw_func(move |_area, cr, _w, _h| {
            let s = state_c.borrow();

            // Grid background
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

            // Local device
            draw_device_rect(cr, &s.local_rect, CORNER_RADIUS, "This Device");

            // Remote device (connected peer)
            if let Some(ref remote) = s.remote {
                if !s.dragging {
                    draw_remote_device(cr, remote, 1.0);
                }
            }

            // Ghost during drag
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

            // Placeholder when disconnected
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

        fn draw_device_rect(cr: &gtk4::cairo::Context, r: &CanvasRect, radius: f64, label: &str) {
            rounded_rect(cr, r.x, r.y, r.w, r.h, radius);
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.25);
            let _ = cr.fill();

            rounded_rect(cr, r.x, r.y, r.w, r.h, radius);
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.8);
            cr.set_line_width(2.0);
            let _ = cr.stroke();

            cr.set_source_rgba(0.208, 0.518, 0.894, 1.0);
            cr.set_font_size(ICON_FONT_SIZE);
            let ext = cr.text_extents(label).unwrap();
            cr.move_to(
                r.x + (r.w - ext.width()) / 2.0,
                r.y + (r.h - ext.height()) / 2.0 + ext.height(),
            );
            let _ = cr.show_text(label);
        }

        fn draw_remote_device(cr: &gtk4::cairo::Context, device: &RemoteDevice, opacity: f64) {
            let r = device.rect;

            rounded_rect(cr, r.x, r.y, r.w, r.h, PEER_CORNER_RADIUS);
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.15 * opacity);
            let _ = cr.fill();

            rounded_rect(cr, r.x, r.y, r.w, r.h, PEER_CORNER_RADIUS);
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.8 * opacity);
            cr.set_line_width(2.0);
            let _ = cr.stroke();

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

    // ── Drag Handling ───────────────────────────────────────────────

    fn setup_drag_handler(
        drawing_area: &gtk4::DrawingArea,
        state: &Rc<RefCell<ArrangementState>>,
        proxy: &Rc<SettingsProxy>,
        continuity: Option<Rc<ContinuityProxy>>,
    ) {
        // Start drag on press
        let click = gtk4::GestureClick::new();
        click.set_button(1);

        let state_press = state.clone();
        click.connect_pressed(move |_gesture, _n_press, px, py| {
            let mut s = state_press.borrow_mut();
            if let Some(ref remote) = s.remote {
                let rect = remote.rect;
                if rect.contains(px, py) {
                    s.dragging = true;
                    s.drag_offset_x = px - rect.x;
                    s.drag_offset_y = py - rect.y;
                    s.drag_pos_x = px;
                    s.drag_pos_y = py;
                }
            }
        });

        drawing_area.add_controller(click);

        // Track motion, snap on button release
        let motion = gtk4::EventControllerMotion::new();

        let state_motion = state.clone();
        let da_motion = drawing_area.clone();
        let proxy_motion = proxy.clone();
        let cont_motion = continuity.clone();
        motion.connect_motion(move |ctrl, px, py| {
            let mut s = state_motion.borrow_mut();
            if s.dragging {
                let still_pressed = ctrl
                    .current_event()
                    .and_then(|ev| {
                        ev.modifier_state()
                            .contains(gtk4::gdk::ModifierType::BUTTON1_MASK)
                            .then_some(true)
                    })
                    .unwrap_or(false);

                if still_pressed {
                    s.drag_pos_x = px;
                    s.drag_pos_y = py;
                    da_motion.queue_draw();
                } else {
                    apply_snap(px, py, &mut s, &proxy_motion, cont_motion.as_ref());
                    s.dragging = false;
                    da_motion.queue_draw();
                }
            }
        });

        let state_leave = state.clone();
        let da_leave = drawing_area.clone();
        let proxy_leave = proxy.clone();
        let cont_leave = continuity;
        motion.connect_leave(move |_ctrl| {
            let mut s = state_leave.borrow_mut();
            if s.dragging {
                let px = s.drag_pos_x;
                let py = s.drag_pos_y;
                apply_snap(px, py, &mut s, &proxy_leave, cont_leave.as_ref());
                s.dragging = false;
                da_leave.queue_draw();
            }
        });

        drawing_area.add_controller(motion);
    }
}
