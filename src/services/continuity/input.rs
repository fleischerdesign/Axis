use async_channel::Sender;
use log::{info, warn};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, AbsoluteAxisCode, KeyCode, RelativeAxisCode, PropType, EventSummary, KeyEvent, RelativeAxisEvent};
use tokio::task::JoinHandle;
use std::time::{Instant, Duration};
use tokio::io::unix::AsyncFd;

use super::protocol::Message;

// ── Input Events ───────────────────────────────────────────────────────

#[derive(Debug)]
pub enum InputEvent {
    CursorMove { dx: f64, dy: f64 },
    KeyPress { key: u32, state: u8 },
    KeyRelease { key: u32 },
    PointerButton { button: u32, state: u8 },
    PointerAxis { dx: f64, dy: f64 },
    EmergencyExit,
}

// ── Input Provider Trait ───────────────────────────────────────────────

pub trait InputCapture: Send {
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<(), String>;
    fn stop(&mut self);
    fn is_capturing(&self) -> bool;
}

pub trait InputInjection: Send {
    fn inject(&mut self, msg: &Message) -> Result<(), String>;
    fn warp(&mut self, side: Side, edge_pos: f64) -> Result<(), String>;
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self);
}

// ── evdev Injection Implementation ──────────────────────────────────────

pub struct WaylandInjection {
    keyboard: Option<VirtualDevice>,
    pointer: Option<VirtualDevice>,
}

impl WaylandInjection {
    pub fn new() -> Self {
        Self { keyboard: None, pointer: None }
    }
}

impl InputInjection for WaylandInjection {
    fn start(&mut self) -> Result<(), String> {
        info!("[continuity:input] creating virtual uinput devices");

        // Device 1: Virtual Keyboard — all key codes
        let mut kb_keys = AttributeSet::<KeyCode>::new();
        for i in 0..512u16 {
            kb_keys.insert(KeyCode::new(i));
        }

        let keyboard = VirtualDevice::builder()
            .map_err(|e| e.to_string())?
            .name("Axis Continuity Virtual Keyboard")
            .with_keys(&kb_keys)
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| format!("failed to build keyboard device: {e}"))?;

        // Device 2: Virtual Pointer — mouse buttons + relative axes + POINTER property
        let mut ptr_keys = AttributeSet::<KeyCode>::new();
        ptr_keys.insert(KeyCode::BTN_LEFT);
        ptr_keys.insert(KeyCode::BTN_RIGHT);
        ptr_keys.insert(KeyCode::BTN_MIDDLE);
        ptr_keys.insert(KeyCode::BTN_SIDE);
        ptr_keys.insert(KeyCode::BTN_EXTRA);

        let mut rel_axes = AttributeSet::<RelativeAxisCode>::new();
        rel_axes.insert(RelativeAxisCode::REL_X);
        rel_axes.insert(RelativeAxisCode::REL_Y);
        rel_axes.insert(RelativeAxisCode::REL_WHEEL);
        rel_axes.insert(RelativeAxisCode::REL_HWHEEL);

        let mut props = AttributeSet::<PropType>::new();
        props.insert(PropType::POINTER);

        let pointer = VirtualDevice::builder()
            .map_err(|e| e.to_string())?
            .name("Axis Continuity Virtual Pointer")
            .with_keys(&ptr_keys)
            .map_err(|e| e.to_string())?
            .with_relative_axes(&rel_axes)
            .map_err(|e| e.to_string())?
            .with_properties(&props)
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| format!("failed to build pointer device: {e}"))?;

        self.keyboard = Some(keyboard);
        self.pointer = Some(pointer);
        Ok(())
    }

    fn stop(&mut self) {
        self.keyboard = None;
        self.pointer = None;
    }

    fn warp(&mut self, side: Side, edge_pos: f64) -> Result<(), String> {
        let ptr = self.pointer.as_mut().ok_or("pointer device not started")?;
        use evdev::InputEvent;

        info!("[continuity:input] warping cursor to {:?} at {:.0}", side, edge_pos);

        // 1. "Slam" the cursor against the entry edge corner to guarantee starting position.
        // We use a delta larger than any reasonable screen resolution.
        let (dx, dy) = match side {
            Side::Left => (-10000, -10000),   // Top-left
            Side::Right => (10000, -10000),   // Top-right
            Side::Top => (-10000, -10000),    // Top-left
            Side::Bottom => (-10000, 10000),  // Bottom-left
        };

        let events = vec![
            InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_X, dx)),
            InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_Y, dy)),
        ];
        ptr.emit(&events).map_err(|e| e.to_string())?;

        // 2. Move from the corner to the actual edge_pos along the edge.
        let mut move_events = Vec::new();
        match side {
            Side::Left | Side::Right => {
                move_events.push(InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_Y, edge_pos as i32)));
            }
            Side::Top | Side::Bottom => {
                move_events.push(InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_X, edge_pos as i32)));
            }
        }
        ptr.emit(&move_events).map_err(|e| e.to_string())?;

        Ok(())
    }

    fn inject(&mut self, msg: &Message) -> Result<(), String> {
        use evdev::InputEvent;

        match msg {
            Message::CursorMove { dx, dy } => {
                let ptr = self.pointer.as_mut().ok_or("pointer device not started")?;
                let dx_i = *dx as i32;
                let dy_i = *dy as i32;
                if dx_i == 0 && dy_i == 0 {
                    return Ok(());
                }
                let mut events = Vec::with_capacity(2);
                if dx_i != 0 {
                    events.push(InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_X, dx_i)));
                }
                if dy_i != 0 {
                    events.push(InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_Y, dy_i)));
                }
                ptr.emit(&events).map_err(|e| e.to_string())?;
            }
            Message::KeyPress { key, state } => {
                let kb = self.keyboard.as_mut().ok_or("keyboard device not started")?;
                let ev = InputEvent::from(KeyEvent::new(KeyCode::new(*key as u16), *state as i32));
                kb.emit(&[ev]).map_err(|e| e.to_string())?;
            }
            Message::KeyRelease { key } => {
                let kb = self.keyboard.as_mut().ok_or("keyboard device not started")?;
                let ev = InputEvent::from(KeyEvent::new(KeyCode::new(*key as u16), 0));
                kb.emit(&[ev]).map_err(|e| e.to_string())?;
            }
            Message::PointerButton { button, state } => {
                let ptr = self.pointer.as_mut().ok_or("pointer device not started")?;
                let ev = InputEvent::from(KeyEvent::new(KeyCode::new(*button as u16), *state as i32));
                ptr.emit(&[ev]).map_err(|e| e.to_string())?;
            }
            Message::PointerAxis { dx, dy } => {
                let ptr = self.pointer.as_mut().ok_or("pointer device not started")?;
                let mut events = Vec::new();
                if *dx != 0.0 {
                    events.push(InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_HWHEEL, *dx as i32)));
                }
                if *dy != 0.0 {
                    events.push(InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_WHEEL, *dy as i32)));
                }
                if !events.is_empty() {
                    ptr.emit(&events).map_err(|e| e.to_string())?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// ── Touchpad ABS→REL Tracker ──────────────────────────────────────────

/// Converts absolute multitouch events (ABS_MT_POSITION_X/Y) from touchpads
/// into relative deltas, since the kernel delivers raw ABS coordinates from
/// touchpads while mice deliver REL. libinput normally does this conversion,
/// but we bypass it by grabbing the device directly.
struct TouchpadTracker {
    last_x: Option<i32>,
    last_y: Option<i32>,
}

impl TouchpadTracker {
    fn new() -> Self {
        Self { last_x: None, last_y: None }
    }

    /// Update position for one axis, return the delta if we had a previous position.
    fn update_x(&mut self, val: i32) -> Option<i32> {
        let delta = self.last_x.map(|prev| val - prev);
        self.last_x = Some(val);
        delta
    }

    fn update_y(&mut self, val: i32) -> Option<i32> {
        let delta = self.last_y.map(|prev| val - prev);
        self.last_y = Some(val);
        delta
    }

    /// Finger lifted — reset tracker so next touch starts fresh.
    fn reset(&mut self) {
        self.last_x = None;
        self.last_y = None;
    }
}

// ── evdev Capture Implementation ───────────────────────────────────────

pub struct EvdevCapture {
    tasks: Vec<JoinHandle<()>>,
}

impl EvdevCapture {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }
}

/// Sensitivity multiplier for converting raw touchpad ABS deltas to pointer deltas.
/// Raw touchpad coordinates span a large range (e.g. 0–4700) so deltas of 1–5 units
/// are common. This factor scales them to feel like normal mouse movement.
/// A typical touchpad width of ~3500 units mapped to ~1920px means ~0.55px per unit,
/// but we want some acceleration so we use a higher factor.
const TOUCHPAD_SENSITIVITY: f64 = 0.75;

impl InputCapture for EvdevCapture {
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<(), String> {
        self.stop();
        info!("[continuity:input] scanning for input devices to grab...");

        let devices = evdev::enumerate();
        let mut grabbed_any = false;
        // Track touchpad base names so we can skip their legacy "Mouse" companion devices
        let mut touchpad_bases: Vec<String> = Vec::new();

        // First pass: identify touchpads
        let devices: Vec<_> = devices.collect();
        for (_path, device) in &devices {
            let name = device.name().unwrap_or("Unknown").to_string();
            let has_mt = device.supported_absolute_axes()
                .map(|a| a.contains(AbsoluteAxisCode::ABS_MT_POSITION_X))
                .unwrap_or(false);
            if has_mt {
                // Extract the base name (e.g. "ELAN06FA:00 04F3:31BE" from "ELAN06FA:00 04F3:31BE Touchpad")
                // The legacy Mouse device is typically named "ELAN06FA:00 04F3:31BE Mouse"
                if let Some(base) = name.strip_suffix(" Touchpad").or_else(|| name.strip_suffix(" Keyboard")) {
                    touchpad_bases.push(base.to_string());
                }
            }
        }

        for (path, mut device) in devices {
            let name = device.name().unwrap_or("Unknown").to_string();
            let name_lower = name.to_lowercase();

            if name_lower.contains("axis continuity") || name_lower.contains("passthrough") || name_lower.contains("virtual") {
                continue;
            }

            // Skip the legacy "Mouse" companion device for touchpads — grabbing the touchpad
            // itself stops it from producing events, so this device is useless and grabbing
            // it just wastes a reader task.
            if name.ends_with(" Mouse") {
                let base = name.strip_suffix(" Mouse").unwrap_or("");
                if touchpad_bases.iter().any(|tb| tb == base) {
                    info!("[continuity:input] skipping legacy touchpad mouse: {} ({})", name, path.display());
                    continue;
                }
            }

            let has_rel_x = device.supported_relative_axes().map(|a| a.contains(RelativeAxisCode::REL_X)).unwrap_or(false);
            let has_enter = device.supported_keys().map(|k| k.contains(KeyCode::KEY_ENTER)).unwrap_or(false);
            let has_mouse_btn = device.supported_keys().map(|k| k.contains(KeyCode::BTN_LEFT)).unwrap_or(false);
            let has_mt = device.supported_absolute_axes()
                .map(|a| a.contains(AbsoluteAxisCode::ABS_MT_POSITION_X))
                .unwrap_or(false);

            if !has_rel_x && !has_enter && !has_mouse_btn {
                continue;
            }

            let is_touchpad = has_mt && has_mouse_btn;

            info!("[continuity:input] grabbing device: {} ({}) [rel_x={} touchpad={} keyboard={}]",
                name, path.display(), has_rel_x, is_touchpad, has_enter && !has_mouse_btn);

            if let Err(e) = device.grab() {
                warn!("[continuity:input] could not grab {}: {}", name, e);
                continue;
            }

            if let Err(e) = device.set_nonblocking(true) {
                warn!("[continuity:input] could not set nonblocking for {}: {}", name, e);
                let _ = device.ungrab();
                continue;
            }

            let mut async_fd = match AsyncFd::new(device) {
                Ok(fd) => fd,
                Err(e) => {
                    warn!("[continuity:input] could not create AsyncFd for {}: {}", name, e);
                    continue;
                }
            };

            grabbed_any = true;
            let tx_c = tx.clone();
            let name_c = name.clone();

            let task = tokio::spawn(async move {
                let mut esc_count = 0;
                let mut last_esc = Instant::now();
                // Touchpad ABS→REL conversion state
                let mut tp_tracker = if is_touchpad { Some(TouchpadTracker::new()) } else { None };

                loop {
                    let mut guard = match async_fd.readable_mut().await {
                        Ok(g) => g,
                        Err(_) => break,
                    };

                    let result = guard.try_io(|fd| {
                        fd.get_mut().fetch_events().map(|events| events.collect::<Vec<_>>())
                    });

                    match result {
                        Ok(Ok(events)) => {
                            for ev in events {
                                match ev.destructure() {
                                    EventSummary::RelativeAxis(_, axis, val) => {
                                        if axis == RelativeAxisCode::REL_X {
                                            let _ = tx_c.send(InputEvent::CursorMove { dx: val as f64, dy: 0.0 }).await;
                                        } else if axis == RelativeAxisCode::REL_Y {
                                            let _ = tx_c.send(InputEvent::CursorMove { dx: 0.0, dy: val as f64 }).await;
                                        } else if axis == RelativeAxisCode::REL_WHEEL {
                                            let _ = tx_c.send(InputEvent::PointerAxis { dx: 0.0, dy: val as f64 }).await;
                                        } else if axis == RelativeAxisCode::REL_HWHEEL {
                                            let _ = tx_c.send(InputEvent::PointerAxis { dx: val as f64, dy: 0.0 }).await;
                                        }
                                    }
                                    EventSummary::AbsoluteAxis(_, axis, val) => {
                                        if let Some(tracker) = tp_tracker.as_mut() {
                                            if axis == AbsoluteAxisCode::ABS_MT_POSITION_X {
                                                if let Some(dx) = tracker.update_x(val) {
                                                    let scaled = dx as f64 * TOUCHPAD_SENSITIVITY;
                                                    let _ = tx_c.send(InputEvent::CursorMove { dx: scaled, dy: 0.0 }).await;
                                                }
                                            } else if axis == AbsoluteAxisCode::ABS_MT_POSITION_Y {
                                                if let Some(dy) = tracker.update_y(val) {
                                                    let scaled = dy as f64 * TOUCHPAD_SENSITIVITY;
                                                    let _ = tx_c.send(InputEvent::CursorMove { dx: 0.0, dy: scaled }).await;
                                                }
                                            } else if axis == AbsoluteAxisCode::ABS_MT_TRACKING_ID && val == -1 {
                                                // Finger lifted
                                                tracker.reset();
                                            }
                                        }
                                    }
                                    EventSummary::Key(_, code, val) => {
                                        let code_u32 = code.code() as u32;

                                        if code == KeyCode::KEY_ESC && val == 1 {
                                            let now = Instant::now();
                                            if now.duration_since(last_esc) < Duration::from_millis(1000) {
                                                esc_count += 1;
                                            } else {
                                                esc_count = 1;
                                            }
                                            last_esc = now;

                                            if esc_count >= 4 {
                                                info!("[continuity:input] KERNEL EMERGENCY EXIT triggered for {}", name_c);
                                                let _ = tx_c.send(InputEvent::EmergencyExit).await;
                                                return;
                                            }
                                        }

                                        let is_mouse = code_u32 >= 272 && code_u32 <= 276;
                                        if is_mouse {
                                            if val == 1 {
                                                let _ = tx_c.send(InputEvent::PointerButton { button: code_u32, state: 1 }).await;
                                            } else if val == 0 {
                                                let _ = tx_c.send(InputEvent::PointerButton { button: code_u32, state: 0 }).await;
                                            }
                                        } else {
                                            if val == 1 || val == 2 {
                                                let _ = tx_c.send(InputEvent::KeyPress { key: code_u32, state: 1 }).await;
                                            } else if val == 0 {
                                                let _ = tx_c.send(InputEvent::KeyRelease { key: code_u32 }).await;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            warn!("[continuity:input] read error for {}: {}", name_c, e);
                            break;
                        }
                        Err(_would_block) => continue,
                    }
                }
                // async_fd (and the Device inside) is dropped here → fd closed → grab released
                info!("[continuity:input] reader thread finished for {}", name_c);
            });
            self.tasks.push(task);
        }

        if !grabbed_any {
            return Err("no suitable input devices found to grab".into());
        }

        Ok(())
    }

    fn stop(&mut self) {
        if !self.tasks.is_empty() {
            info!("[continuity:input] stopping {} reader tasks", self.tasks.len());
            for task in self.tasks.drain(..) {
                task.abort();
            }
        }
    }

    fn is_capturing(&self) -> bool {
        !self.tasks.is_empty()
    }
}
