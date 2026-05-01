use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use axis_domain::models::continuity::{ContinuityStatus, PeerArrangement, Side};

const CANVAS_VIRTUAL_WIDTH: f64 = 500.0;
const CANVAS_VIRTUAL_HEIGHT: f64 = 350.0;
const CANVAS_GAP: f64 = 15.0;
const CORNER_RADIUS: f64 = 10.0;
const PEER_CORNER_RADIUS: f64 = 8.0;
const LABEL_FONT_SIZE: f64 = 11.0;
const ICON_FONT_SIZE: f64 = 16.0;
const DRAG_OPACITY: f64 = 0.5;
const CANVAS_PADDING: f64 = 30.0;

#[derive(Clone)]
struct RemoteDevice {
    #[allow(dead_code)]
    device_id: String,
    device_name: String,
    side: Side,
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
    local_screen: (i32, i32),
    peer_screen: (i32, i32),
    dragging: bool,
    drag_offset_x: f64,
    drag_offset_y: f64,
    drag_pos_x: f64,
    drag_pos_y: f64,
    potential_side: Option<Side>,
    skip_update: bool,
}

fn compute_layout(
    local_w: i32,
    local_h: i32,
    peer_w: i32,
    peer_h: i32,
) -> (CanvasRect, f64, CanvasRect) {
    let lw = local_w as f64;
    let lh = local_h as f64;
    let pw = peer_w as f64;
    let ph = peer_h as f64;
    let avail_w = CANVAS_VIRTUAL_WIDTH - 2.0 * CANVAS_PADDING;
    let avail_h = CANVAS_VIRTUAL_HEIGHT - 2.0 * CANVAS_PADDING;
    let total_w = lw + CANVAS_GAP + pw;
    let total_h = lh.max(ph);
    let scale = (avail_w / total_w).min(avail_h / total_h).max(0.01);
    let slw = lw * scale;
    let slh = lh * scale;
    let spw = pw * scale;
    let sph = ph * scale;
    let combined_w = slw + CANVAS_GAP + spw;
    let combined_h = slh.max(sph);
    let origin_x = (CANVAS_VIRTUAL_WIDTH - combined_w) / 2.0;
    let origin_y = (CANVAS_VIRTUAL_HEIGHT - combined_h) / 2.0;
    let local_rect = CanvasRect {
        x: origin_x,
        y: origin_y + (combined_h - slh) / 2.0,
        w: slw,
        h: slh,
    };
    let peer_rect = CanvasRect {
        x: local_rect.x + slw + CANVAS_GAP,
        y: local_rect.y,
        w: spw,
        h: sph,
    };
    (local_rect, scale, peer_rect)
}

fn canvas_rect_from_arrangement(
    side: Side,
    offset: i32,
    local: &CanvasRect,
    peer_w: f64,
    peer_h: f64,
    scale: f64,
) -> CanvasRect {
    let offset_canvas = offset as f64 * scale;
    match side {
        Side::Right => CanvasRect {
            x: local.x + local.w + CANVAS_GAP,
            y: local.y + offset_canvas,
            w: peer_w,
            h: peer_h,
        },
        Side::Left => CanvasRect {
            x: local.x - peer_w - CANVAS_GAP,
            y: local.y + offset_canvas,
            w: peer_w,
            h: peer_h,
        },
        Side::Top => CanvasRect {
            x: local.x + offset_canvas,
            y: local.y - peer_h - CANVAS_GAP,
            w: peer_w,
            h: peer_h,
        },
        Side::Bottom => CanvasRect {
            x: local.x + offset_canvas,
            y: local.y + local.h + CANVAS_GAP,
            w: peer_w,
            h: peer_h,
        },
    }
}

fn calculate_snap(px: f64, py: f64, state: &ArrangementState) -> (Side, i32, CanvasRect) {
    let local = state.local_rect;
    let scale = state.scale;
    let pw = state.peer_rect.w;
    let ph = state.peer_rect.h;
    let peer_x = px - state.drag_offset_x;
    let peer_y = py - state.drag_offset_y;
    let peer_cx = peer_x + pw / 2.0;
    let peer_cy = peer_y + ph / 2.0;
    let local_cx = local.x + local.w / 2.0;
    let local_cy = local.y + local.h / 2.0;
    let dx = peer_cx - local_cx;
    let dy = peer_cy - local_cy;
    let side = if (dx / local.w).abs() > (dy / local.h).abs() {
        if dx > 0.0 { Side::Right } else { Side::Left }
    } else {
        if dy > 0.0 { Side::Bottom } else { Side::Top }
    };
    let snapped = canvas_rect_from_arrangement(side, 0, &local, pw, ph, scale);
    let snapped = match side {
        Side::Right | Side::Left => CanvasRect { y: peer_y, ..snapped },
        Side::Top | Side::Bottom => CanvasRect { x: peer_x, ..snapped },
    };
    let offset = match side {
        Side::Right | Side::Left => ((snapped.y - local.y) / scale).round() as i32,
        Side::Top | Side::Bottom => ((snapped.x - local.x) / scale).round() as i32,
    };
    (side, offset, snapped)
}

pub struct ArrangementGrid {
    drawing_area: gtk4::DrawingArea,
    state: Rc<RefCell<ArrangementState>>,
}

impl ArrangementGrid {
    pub fn new(on_snap: impl Fn(PeerArrangement) + 'static) -> Rc<Self> {
        let drawing_area = gtk4::DrawingArea::builder()
            .css_classes(vec!["arrangement-grid".to_string()])
            .hexpand(true)
            .vexpand(true)
            .halign(gtk4::Align::Fill)
            .valign(gtk4::Align::Fill)
            .can_target(true)
            .build();

        drawing_area.set_content_width(CANVAS_VIRTUAL_WIDTH as i32);
        drawing_area.set_content_height(CANVAS_VIRTUAL_HEIGHT as i32);

        let default_screen: (i32, i32) = (1920, 1080);
        let (local_rect, scale, peer_rect) = compute_layout(
            default_screen.0, default_screen.1,
            default_screen.0, default_screen.1,
        );

        let state = Rc::new(RefCell::new(ArrangementState {
            remote: None,
            local_rect,
            peer_rect,
            scale,
            local_screen: default_screen,
            peer_screen: default_screen,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            drag_pos_x: 0.0,
            drag_pos_y: 0.0,
            potential_side: None,
            skip_update: false,
        }));

        Self::setup_draw_func(&drawing_area, &state);
        Self::setup_drag_handler(&drawing_area, &state, on_snap);

        Rc::new(Self { drawing_area, state })
    }

    pub fn widget(&self) -> &gtk4::DrawingArea {
        &self.drawing_area
    }

    pub fn update_status(&self, status: &ContinuityStatus) {
        let mut s = self.state.borrow_mut();

        let new_local = (status.screen_width, status.screen_height);
        let new_peer = status.remote_screen.unwrap_or(new_local);
        let dims_changed = new_local != s.local_screen || new_peer != s.peer_screen;

        if dims_changed && !s.dragging {
            let (local_rect, scale, peer_rect) = compute_layout(
                new_local.0, new_local.1, new_peer.0, new_peer.1,
            );
            s.local_rect = local_rect;
            s.peer_rect = peer_rect;
            s.scale = scale;
            s.local_screen = new_local;
            s.peer_screen = new_peer;
        }

        if s.skip_update {
            if let Some(ref conn) = status.active_connection {
                let pc = status.peer_configs.get(&conn.peer_id);
                let incoming = pc.map(|p| p.arrangement).unwrap_or_default();
                if let Some(ref remote) = s.remote {
                    if incoming.side == remote.side && incoming.offset == remote.offset {
                        drop(s);
                        return;
                    }
                }
            }
            s.skip_update = false;
        }

        let local = s.local_rect;
        let scale = s.scale;
        let pw = s.peer_rect.w;
        let ph = s.peer_rect.h;

        s.remote = if let Some(ref conn) = status.active_connection {
            let pc = status.peer_configs.get(&conn.peer_id);
            let arrangement = pc.map(|p| p.arrangement).unwrap_or_default();
            let side = arrangement.side;
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

        drop(s);
        self.drawing_area.queue_draw();
    }

    fn setup_draw_func(drawing_area: &gtk4::DrawingArea, state: &Rc<RefCell<ArrangementState>>) {
        let state_c = state.clone();
        drawing_area.set_draw_func(move |_area, cr, _w, _h| {
            let s = state_c.borrow();

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

            draw_device_rect(cr, &s.local_rect, CORNER_RADIUS, "This Device");

            if s.dragging {
                if let Some(side) = s.potential_side {
                    draw_snap_highlight(cr, &s.local_rect, side);
                }
            }

            if let Some(ref remote) = s.remote {
                if !s.dragging {
                    draw_remote_device(cr, remote, 1.0);
                }
            }

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

        fn draw_snap_highlight(cr: &gtk4::cairo::Context, r: &CanvasRect, side: Side) {
            let thickness = 4.0;
            let gap_half = CANVAS_GAP / 2.0;
            cr.set_source_rgba(0.208, 0.518, 0.894, 0.6);
            cr.set_line_width(thickness);
            cr.set_line_cap(gtk4::cairo::LineCap::Round);
            match side {
                Side::Left => {
                    let x = r.x - gap_half;
                    cr.move_to(x, r.y);
                    cr.line_to(x, r.y + r.h);
                }
                Side::Right => {
                    let x = r.x + r.w + gap_half;
                    cr.move_to(x, r.y);
                    cr.line_to(x, r.y + r.h);
                }
                Side::Top => {
                    let y = r.y - gap_half;
                    cr.move_to(r.x, y);
                    cr.line_to(r.x + r.w, y);
                }
                Side::Bottom => {
                    let y = r.y + r.h + gap_half;
                    cr.move_to(r.x, y);
                    cr.line_to(r.x + r.w, y);
                }
            }
            let _ = cr.stroke();
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

    fn setup_drag_handler(
        drawing_area: &gtk4::DrawingArea,
        state: &Rc<RefCell<ArrangementState>>,
        on_snap: impl Fn(PeerArrangement) + 'static,
    ) {
        let on_snap = Rc::new(on_snap);

        let click = gtk4::GestureClick::new();
        click.set_button(1);

        let state_press = state.clone();
        click.connect_pressed(move |_gesture, _n, px, py| {
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

        let state_release = state.clone();
        let snap_release = on_snap.clone();
        let da_release = drawing_area.clone();
        click.connect_released(move |_gesture, _n, px, py| {
            let mut s = state_release.borrow_mut();
            if s.dragging {
                let (side, offset, snapped) = calculate_snap(px, py, &s);
                if let Some(ref mut remote) = s.remote {
                    remote.side = side;
                    remote.offset = offset;
                    remote.rect = snapped;
                }
                s.potential_side = None;
                s.skip_update = true;
                snap_release(PeerArrangement { side, offset });
                s.dragging = false;
                da_release.queue_draw();
            }
        });

        drawing_area.add_controller(click);

        let motion = gtk4::EventControllerMotion::new();
        let state_motion = state.clone();
        let da_motion = drawing_area.clone();
        let snap_motion = on_snap.clone();

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
                    let (side, _, _) = calculate_snap(px, py, &s);
                    s.potential_side = Some(side);
                    da_motion.queue_draw();
                } else {
                    let (side, offset, snapped) = calculate_snap(px, py, &s);
                    if let Some(ref mut remote) = s.remote {
                        remote.side = side;
                        remote.offset = offset;
                        remote.rect = snapped;
                    }
                    s.potential_side = None;
                    s.skip_update = true;
                    snap_motion(PeerArrangement { side, offset });
                    s.dragging = false;
                    da_motion.queue_draw();
                }
            }
        });

        let state_leave = state.clone();
        let da_leave = drawing_area.clone();
        let snap_leave = on_snap;
        motion.connect_leave(move |_ctrl| {
            let mut s = state_leave.borrow_mut();
            if s.dragging {
                let px = s.drag_pos_x;
                let py = s.drag_pos_y;
                let (side, offset, snapped) = calculate_snap(px, py, &s);
                if let Some(ref mut remote) = s.remote {
                    remote.side = side;
                    remote.offset = offset;
                    remote.rect = snapped;
                }
                s.potential_side = None;
                s.skip_update = true;
                snap_leave(PeerArrangement { side, offset });
                s.dragging = false;
                da_leave.queue_draw();
            }
        });

        drawing_area.add_controller(motion);
    }
}
