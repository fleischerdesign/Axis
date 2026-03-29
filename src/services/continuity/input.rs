use async_channel::Sender;
use log::{info, warn, debug};
use evdev::uinput::VirtualDeviceBuilder;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Key, InputEvent as EvEvent, EventType, RelativeAxisType};
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
        info!("[continuity:input] scanning for input devices to grab...");

        let devices = evdev::enumerate();
        let mut grabbed_any = false;
        let mut found_count = 0;

        for (path, mut device) in devices {
            found_count += 1;
            let name = device.name().unwrap_or("Unknown").to_string();
            
            if name.contains("Axis Continuity") { 
                debug!("[continuity:input] skipping own virtual device: {}", name);
                continue; 
            }

            let has_rel_x = device.supported_relative_axes().map(|a| a.contains(RelativeAxisType::REL_X)).unwrap_or(false);
            let has_enter = device.supported_keys().map(|k| k.contains(Key::KEY_ENTER)).unwrap_or(false);
            let has_mouse_btn = device.supported_keys().map(|k| k.contains(Key::BTN_LEFT)).unwrap_or(false);

            debug!(
                "[continuity:input] found device: {} (rel_x={}, enter={}, mouse_btn={})",
                name, has_rel_x, has_enter, has_mouse_btn
            );

            if !has_rel_x && !has_enter && !has_mouse_btn {
                continue;
            }

            info!("[continuity:input] grabbing device: {} ({})", name, path.display());

            if let Err(e) = device.grab() {
                warn!("[continuity:input] could not grab {}: {} (check group permissions)", name, e);
                continue;
            }

            grabbed_any = true;
            let tx_c = tx.clone();
            let name_c = name.clone();
            
            let task = tokio::task::spawn_blocking(move || {
                info!("[continuity:input] reader thread started for {}", name_c);
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
                                        } else if axis == RelativeAxisType::REL_WHEEL {
                                            let _ = tx_c.send_blocking(InputEvent::PointerAxis { dx: 0.0, dy: ev.value() as f64 });
                                        } else if axis == RelativeAxisType::REL_WHEEL_HI_RES {
                                            // Optional: Handle hi-res scrolling
                                        } else if axis == RelativeAxisType::REL_HWHEEL {
                                            let _ = tx_c.send_blocking(InputEvent::PointerAxis { dx: ev.value() as f64, dy: 0.0 });
                                        }
                                    }
                                    EventType::KEY => {
                                        let code = ev.code() as u32;
                                        let val = ev.value();
                                        let is_mouse = code >= 272 && code <= 276;
                                        
                                        if is_mouse {
                                            if val == 1 {
                                                let _ = tx_c.send_blocking(InputEvent::PointerButton { button: code, state: 1 });
                                            } else if val == 0 {
                                                let _ = tx_c.send_blocking(InputEvent::PointerButton { button: code, state: 0 });
                                            }
                                        } else {
                                            if val == 1 || val == 2 {
                                                let _ = tx_c.send_blocking(InputEvent::KeyPress { key: code, state: 1 });
                                            } else if val == 0 {
                                                let _ = tx_c.send_blocking(InputEvent::KeyRelease { key: code });
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => {
                            warn!("[continuity:input] reader error for {}: {}", name_c, e);
                            break;
                        }
                    }
                }
            });
            self.tasks.push(task);
        }

        if found_count == 0 {
            return Err("no devices found in /dev/input (check permissions/group)".into());
        }

        if !grabbed_any {
            return Err("no suitable keyboards or mice found to grab".into());
        }

        Ok(())
    }

    fn stop(&mut self) {
        if !self.tasks.is_empty() {
            info!("[continuity:input] releasing {} grabbed devices", self.tasks.len());
            for task in self.tasks.drain(..) {
                task.abort();
            }
        }
    }

    fn is_capturing(&self) -> bool {
        !self.tasks.is_empty()
    }
}
