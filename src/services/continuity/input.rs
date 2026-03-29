use async_channel::Sender;
use log::{info, warn};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, KeyCode, RelativeAxisCode, PropType, EventSummary, KeyEvent, RelativeAxisEvent};
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
        // BTN_LEFT+REL_X+REL_Y is the canonical mouse signature libinput uses to classify
        // a device as a pointer. Without these, libinput may classify it as keyboard
        // and drop REL events (the 512 key codes confuse the classifier).
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

    fn inject(&mut self, msg: &Message) -> Result<(), String> {
        use evdev::InputEvent;

        match msg {
            Message::CursorMove { dx, dy } => {
                let ptr = self.pointer.as_mut().ok_or("pointer device not started")?;
                let dx_i = *dx as i32;
                let dy_i = *dy as i32;
                info!("[continuity:inject] CursorMove dx={} dy={} (i32: dx={} dy={})", dx, dy, dx_i, dy_i);
                let events = [
                    InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_X, dx_i)),
                    InputEvent::from(RelativeAxisEvent::new(RelativeAxisCode::REL_Y, dy_i)),
                ];
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

// ── evdev Capture Implementation ───────────────────────────────────────

pub struct EvdevCapture {
    tasks: Vec<JoinHandle<()>>,
}

impl EvdevCapture {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }
}

impl InputCapture for EvdevCapture {
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<(), String> {
        self.stop();
        info!("[continuity:input] scanning for input devices to grab...");

        let devices = evdev::enumerate();
        let mut grabbed_any = false;

        for (path, mut device) in devices {
            let name = device.name().unwrap_or("Unknown").to_string();
            let name_lower = name.to_lowercase();

            if name_lower.contains("axis continuity") || name_lower.contains("passthrough") || name_lower.contains("virtual") {
                continue;
            }

            let has_rel_x = device.supported_relative_axes().map(|a| a.contains(RelativeAxisCode::REL_X)).unwrap_or(false);
            let has_enter = device.supported_keys().map(|k| k.contains(KeyCode::KEY_ENTER)).unwrap_or(false);
            let has_mouse_btn = device.supported_keys().map(|k| k.contains(KeyCode::BTN_LEFT)).unwrap_or(false);

            if !has_rel_x && !has_enter && !has_mouse_btn {
                continue;
            }

            let has_abs = device.supported_absolute_axes().map(|a| !a.iter().next().is_none()).unwrap_or(false);
            info!("[continuity:input] grabbing device: {} ({}) [rel_x={} enter={} btn_left={} has_abs={}]",
                name, path.display(), has_rel_x, has_enter, has_mouse_btn, has_abs);

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
                                        info!("[continuity:capture] {} REL event: axis={:?} val={}", name_c, axis, val);
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
                                        info!("[continuity:capture] {} ABS event: axis={:?} val={}", name_c, axis, val);
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
