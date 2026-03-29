use async_channel::Sender;
use log::{error, info, warn};
use evdev::uinput::VirtualDeviceBuilder;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Key, InputEvent as EvEvent, EventType, RelativeAxisType, Device};
use std::sync::Arc;
use tokio::task::JoinHandle;

use super::protocol::Message;

// ── Input Events ───────────────────────────────────────────────────────

#[derive(Debug)]
pub enum InputEvent {
    CursorMove { dx: f64, dy: f64 },
    KeyPress { key: u32, state: u8 },
    KeyRelease { key: u32 },
    PointerButton { button: u32, state: u8 },
    PointerAxis { dx: f64, dy: f64 },
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
    device: Option<VirtualDevice>,
}

impl WaylandInjection {
    pub fn new() -> Self {
        Self { device: None }
    }
}

impl InputInjection for WaylandInjection {
    fn start(&mut self) -> Result<(), String> {
        info!("[continuity:input] creating virtual uinput device");
        
        let mut keys = AttributeSet::<Key>::new();
        for i in 0..512 {
            keys.insert(Key::new(i));
        }

        let mut rel_axes = AttributeSet::<RelativeAxisType>::new();
        rel_axes.insert(RelativeAxisType::REL_X);
        rel_axes.insert(RelativeAxisType::REL_Y);
        rel_axes.insert(RelativeAxisType::REL_WHEEL);
        rel_axes.insert(RelativeAxisType::REL_HWHEEL);

        let device = VirtualDeviceBuilder::new()
            .map_err(|e| e.to_string())?
            .name("Axis Continuity Virtual Input")
            .with_keys(&keys)
            .map_err(|e| e.to_string())?
            .with_relative_axes(&rel_axes)
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| format!("failed to build virtual device: {e}"))?;

        self.device = Some(device);
        Ok(())
    }

    fn stop(&mut self) {
        self.device = None;
    }

    fn inject(&mut self, msg: &Message) -> Result<(), String> {
        let dev = self.device.as_mut().ok_or("injection not started")?;

        match msg {
            Message::CursorMove { dx, dy } => {
                let events = [
                    EvEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, *dx as i32),
                    EvEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, *dy as i32),
                ];
                dev.emit(&events).map_err(|e| e.to_string())?;
            }
            Message::KeyPress { key, state } => {
                let ev = EvEvent::new(EventType::KEY, *key as u16, *state as i32);
                dev.emit(&[ev]).map_err(|e| e.to_string())?;
            }
            Message::KeyRelease { key } => {
                let ev = EvEvent::new(EventType::KEY, *key as u16, 0);
                dev.emit(&[ev]).map_err(|e| e.to_string())?;
            }
            Message::PointerButton { button, state } => {
                let ev = EvEvent::new(EventType::KEY, *button as u16, *state as i32);
                dev.emit(&[ev]).map_err(|e| e.to_string())?;
            }
            Message::PointerAxis { dx, dy } => {
                let mut events = Vec::new();
                if *dx != 0.0 {
                    events.push(EvEvent::new(EventType::RELATIVE, RelativeAxisType::REL_HWHEEL.0, *dx as i32));
                }
                if *dy != 0.0 {
                    events.push(EvEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, *dy as i32));
                }
                if !events.is_empty() {
                    dev.emit(&events).map_err(|e| e.to_string())?;
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
        info!("[continuity:input] starting evdev capture (EVIOCGRAB)");

        let devices = evdev::enumerate();
        for (path, mut device) in devices {
            // Skip our own virtual device
            if let Some(name) = device.name() {
                if name.contains("Axis Continuity") { continue; }
            }

            // We only want keyboards and mice/pointing devices
            let is_keyboard = device.supported_keys().map(|k| k.contains(Key::KEY_ENTER)).unwrap_or(false);
            let is_mouse = device.supported_relative_axes().map(|a| a.contains(RelativeAxisType::REL_X)).unwrap_or(false);

            if !is_keyboard && !is_mouse { continue; }

            info!("[continuity:input] grabbing device: {:?} ({})", device.name(), path.display());

            if let Err(e) = device.grab() {
                warn!("[continuity:input] failed to grab {:?}: {}", path, e);
                continue;
            }

            let tx_c = tx.clone();
            let task = tokio::task::spawn_blocking(move || {
                loop {
                    match device.fetch_events() {
                        Ok(events) => {
                            for ev in events {
                                match ev.event_type() {
                                    EventType::RELATIVE => {
                                        let axis = RelativeAxisType(ev.code());
                                        if axis == RelativeAxisType::REL_X {
                                            let _ = tx_c.send_blocking(InputEvent::CursorMove { dx: ev.value() as f64, dy: 0.0 });
                                        } else if axis == RelativeAxisType::REL_Y {
                                            let _ = tx_c.send_blocking(InputEvent::CursorMove { dx: 0.0, dy: ev.value() as f64 });
                                        }
                                    }
                                    EventType::KEY => {
                                        let key = ev.code() as u32;
                                        let is_mouse_button = key >= 272 && key <= 276; // BTN_MOUSE, BTN_RIGHT, BTN_MIDDLE, etc.
                                        
                                        if is_mouse_button {
                                            if ev.value() == 1 {
                                                let _ = tx_c.send_blocking(InputEvent::PointerButton { button: key, state: 1 });
                                            } else if ev.value() == 0 {
                                                let _ = tx_c.send_blocking(InputEvent::PointerButton { button: key, state: 0 });
                                            }
                                        } else {
                                            if ev.value() == 1 || ev.value() == 2 { // Pressed or Repeat
                                                let _ = tx_c.send_blocking(InputEvent::KeyPress { key, state: 1 });
                                            } else if ev.value() == 0 { // Released
                                                let _ = tx_c.send_blocking(InputEvent::KeyRelease { key });
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => {
                            warn!("[continuity:input] device disconnected or error: {}", e);
                            break;
                        }
                    }
                }
            });
            self.tasks.push(task);
        }

        Ok(())
    }

    fn stop(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }
        // Note: Grabs are automatically released by the kernel when the file descriptor is closed (on drop)
    }

    fn is_capturing(&self) -> bool {
        !self.tasks.is_empty()
    }
}
