use async_channel::Sender;
use axis_domain::models::continuity::{Message, Side};
use log::{info, warn};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, AbsoluteAxisCode, KeyCode, RelativeAxisCode, PropType, EventSummary, KeyEvent, RelativeAxisEvent};
use tokio::task::JoinHandle;
use std::time::{Instant, Duration};
use tokio::io::unix::AsyncFd;

#[derive(Debug)]
pub enum InternalInputEvent {
    CursorMove { dx: f64, dy: f64 },
    KeyPress { key: u32, state: u8 },
    KeyRelease { key: u32 },
    PointerButton { button: u32, state: u8 },
    PointerAxis { dx: f64, dy: f64 },
    EmergencyExit,
}

pub trait InputCapture: Send {
    fn prepare(&mut self) -> Result<(), String>;
    fn start(&mut self, tx: Sender<InternalInputEvent>) -> Result<(), String>;
    fn stop(&mut self);
    fn is_capturing(&self) -> bool;
}

pub trait InputInjection: Send {
    fn inject(&mut self, msg: &Message) -> Result<(), String>;
    fn warp(&mut self, side: Side, edge_pos: f64, screen_w: i32, screen_h: i32) -> Result<(), String>;
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self);
}

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

        use evdev::{UinputAbsSetup, AbsInfo};

        let x_setup = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_X,
            AbsInfo::new(0, 0, 32767, 0, 0, 0)
        );
        let y_setup = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_Y,
            AbsInfo::new(0, 0, 32767, 0, 0, 0)
        );

        let mut props = AttributeSet::<PropType>::new();
        props.insert(PropType::POINTER);

        let pointer = VirtualDevice::builder()
            .map_err(|e| e.to_string())?
            .name("Axis Continuity Virtual Pointer")
            .with_keys(&ptr_keys)
            .map_err(|e| e.to_string())?
            .with_relative_axes(&rel_axes)
            .map_err(|e| e.to_string())?
            .with_absolute_axis(&x_setup)
            .map_err(|e| e.to_string())?
            .with_absolute_axis(&y_setup)
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

    fn warp(&mut self, side: Side, edge_pos: f64, screen_w: i32, screen_h: i32) -> Result<(), String> {
        let ptr = self.pointer.as_mut().ok_or("pointer device not started")?;
        use evdev::AbsoluteAxisEvent;

        let (px, py) = match side {
            Side::Left => (1.0, edge_pos),
            Side::Right => (screen_w as f64 - 1.0, edge_pos),
            Side::Top => (edge_pos, 1.0),
            Side::Bottom => (edge_pos, screen_h as f64 - 1.0),
        };

        let x = (px / screen_w as f64 * 32767.0) as i32;
        let y = (py / screen_h as f64 * 32767.0) as i32;

        info!("[continuity:input] warping cursor to {:?} at pixel ({:.0}, {:.0}) -> abs ({}, {})", side, px, py, x, y);

        let events = vec![
            evdev::InputEvent::from(AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_X, x)),
            evdev::InputEvent::from(AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_Y, y)),
        ];
        ptr.emit(&events).map_err(|e| e.to_string())?;

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

struct TouchpadTracker {
    last_x: Option<i32>,
    last_y: Option<i32>,
}

impl TouchpadTracker {
    fn new() -> Self {
        Self { last_x: None, last_y: None }
    }

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

    fn reset(&mut self) {
        self.last_x = None;
        self.last_y = None;
    }
}

pub struct EvdevCapture {
    tasks: Vec<JoinHandle<()>>,
    prepared_devices: Vec<(evdev::Device, bool)>,
}

impl EvdevCapture {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            prepared_devices: Vec::new(),
        }
    }
}

const TOUCHPAD_SENSITIVITY: f64 = 0.75;
const EMERGENCY_EXIT_PRESSES: u32 = 4;
const EMERGENCY_EXIT_WINDOW_MS: u64 = 1000;

const VIRTUAL_DEVICE_PREFIXES: &[&str] = &[
    "axis continuity virtual",
    "axis continuity",
];

const VIRTUAL_DEVICE_EXACT: &[&str] = &[
    "axis continuity virtual keyboard",
    "axis continuity virtual pointer",
];

fn is_virtual_device(name: &str) -> bool {
    let lower = name.to_lowercase();
    VIRTUAL_DEVICE_EXACT.iter().any(|n| lower == *n)
        || VIRTUAL_DEVICE_PREFIXES.iter().any(|p| lower.starts_with(p))
}

impl InputCapture for EvdevCapture {
    fn prepare(&mut self) -> Result<(), String> {
        let start_time = Instant::now();
        self.prepared_devices.clear();
        info!("[continuity:input] pre-scanning input devices...");

        let devices = evdev::enumerate();
        let mut touchpad_bases: Vec<String> = Vec::new();

        let devices_collected: Vec<_> = devices.collect();
        let enum_duration = start_time.elapsed();
        info!("[continuity:input] enumeration took {:?}", enum_duration);

        for (_path, device) in &devices_collected {
            let name = device.name().unwrap_or("Unknown").to_string();
            let has_mt = device.supported_absolute_axes()
                .map(|a| a.contains(AbsoluteAxisCode::ABS_MT_POSITION_X))
                .unwrap_or(false);
            if has_mt {
                if let Some(base) = name.strip_suffix(" Touchpad").or_else(|| name.strip_suffix(" Keyboard")) {
                    touchpad_bases.push(base.to_string());
                }
            }
        }

        for (path, device) in devices_collected {
            let name = device.name().unwrap_or("Unknown").to_string();

            if is_virtual_device(&name) {
                continue;
            }

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
            self.prepared_devices.push((device, is_touchpad));
        }

        info!("[continuity:input] prepared {} devices in {:?}", self.prepared_devices.len(), start_time.elapsed());
        Ok(())
    }

    fn start(&mut self, tx: Sender<InternalInputEvent>) -> Result<(), String> {
        let start_time = Instant::now();
        self.stop();

        if self.prepared_devices.is_empty() {
            let _ = self.prepare();
        }

        if self.prepared_devices.is_empty() {
            return Err("no suitable input devices found to grab".into());
        }

        let devices = std::mem::take(&mut self.prepared_devices);
        let mut grabbed_any = false;

        for (mut device, is_touchpad) in devices {
            let grab_start = Instant::now();
            let name = device.name().unwrap_or("Unknown").to_string();

            if let Err(e) = device.grab() {
                warn!("[continuity:input] could not grab {}: {} (after {:?})", name, e, grab_start.elapsed());
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
                let mut tp_tracker = if is_touchpad { Some(TouchpadTracker::new()) } else { None };

                loop {
                    let mut guard = match async_fd.readable_mut().await {
                        Ok(g) => g,
                        Err(_) => break,
                    };

                    let events: Vec<_> = match guard.try_io(|fd| {
                        fd.get_mut().fetch_events().map(|events| events.collect())
                    }) {
                        Ok(Ok(events)) => events,
                        Ok(Err(e)) => {
                            warn!("[continuity:input] read error for {}: {}", name_c, e);
                            break;
                        }
                        Err(_) => continue,
                    };

                    for ev in events {
                        match ev.destructure() {
                            EventSummary::RelativeAxis(_, axis, val) => {
                                if axis == RelativeAxisCode::REL_X {
                                    let _ = tx_c.send(InternalInputEvent::CursorMove { dx: val as f64, dy: 0.0 }).await;
                                } else if axis == RelativeAxisCode::REL_Y {
                                    let _ = tx_c.send(InternalInputEvent::CursorMove { dx: 0.0, dy: val as f64 }).await;
                                } else if axis == RelativeAxisCode::REL_WHEEL {
                                    let _ = tx_c.send(InternalInputEvent::PointerAxis { dx: 0.0, dy: val as f64 }).await;
                                } else if axis == RelativeAxisCode::REL_HWHEEL {
                                    let _ = tx_c.send(InternalInputEvent::PointerAxis { dx: val as f64, dy: 0.0 }).await;
                                }
                            }
                            EventSummary::AbsoluteAxis(_, axis, val) => {
                                if let Some(tracker) = tp_tracker.as_mut() {
                                    if axis == AbsoluteAxisCode::ABS_MT_POSITION_X {
                                        if let Some(dx) = tracker.update_x(val) {
                                            let scaled = dx as f64 * TOUCHPAD_SENSITIVITY;
                                            let _ = tx_c.send(InternalInputEvent::CursorMove { dx: scaled, dy: 0.0 }).await;
                                        }
                                    } else if axis == AbsoluteAxisCode::ABS_MT_POSITION_Y {
                                        if let Some(dy) = tracker.update_y(val) {
                                            let scaled = dy as f64 * TOUCHPAD_SENSITIVITY;
                                            let _ = tx_c.send(InternalInputEvent::CursorMove { dx: 0.0, dy: scaled }).await;
                                        }
                                    } else if axis == AbsoluteAxisCode::ABS_MT_TRACKING_ID && val == -1 {
                                        tracker.reset();
                                    }
                                }
                            }
                            EventSummary::Key(_, code, val) => {
                                let code_u32 = code.code() as u32;

                                if code == KeyCode::KEY_ESC && val == 1 {
                                    let now = Instant::now();
                                    if now.duration_since(last_esc) < Duration::from_millis(EMERGENCY_EXIT_WINDOW_MS) {
                                        esc_count += 1;
                                    } else {
                                        esc_count = 1;
                                    }
                                    last_esc = now;

                                    if esc_count >= EMERGENCY_EXIT_PRESSES {
                                        info!("[continuity:input] KERNEL EMERGENCY EXIT triggered for {}", name_c);
                                        let _ = tx_c.send(InternalInputEvent::EmergencyExit).await;
                                        return;
                                    }
                                }

                                let is_mouse = code_u32 >= 272 && code_u32 <= 276;
                                if is_mouse {
                                    if val == 1 {
                                        let _ = tx_c.send(InternalInputEvent::PointerButton { button: code_u32, state: 1 }).await;
                                    } else if val == 0 {
                                        let _ = tx_c.send(InternalInputEvent::PointerButton { button: code_u32, state: 0 }).await;
                                    }
                                } else {
                                    if val == 1 || val == 2 {
                                        let _ = tx_c.send(InternalInputEvent::KeyPress { key: code_u32, state: 1 }).await;
                                    } else if val == 0 {
                                        let _ = tx_c.send(InternalInputEvent::KeyRelease { key: code_u32 }).await;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                info!("[continuity:input] reader thread finished for {}", name_c);
            });
            self.tasks.push(task);
            info!("[continuity:input] grabbed {} in {:?}", name, grab_start.elapsed());
        }

        info!("[continuity:input] total start took {:?}", start_time.elapsed());
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
