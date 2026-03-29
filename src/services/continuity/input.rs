use async_channel::Sender;
use log::{info, warn};
use evdev::uinput::VirtualDeviceBuilder;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Key, InputEvent as EvEvent, EventType, RelativeAxisType};
use tokio::task::JoinHandle;
use std::time::{Instant, Duration};
use futures_util::StreamExt;

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

        for (path, mut device) in devices {
            let name = device.name().unwrap_or("Unknown").to_string();
            let name_lower = name.to_lowercase();
            
            if name_lower.contains("axis continuity") || name_lower.contains("passthrough") || name_lower.contains("virtual") { 
                continue; 
            }

            let has_rel_x = device.supported_relative_axes().map(|a| a.contains(RelativeAxisType::REL_X)).unwrap_or(false);
            let has_enter = device.supported_keys().map(|k| k.contains(Key::KEY_ENTER)).unwrap_or(false);
            let has_mouse_btn = device.supported_keys().map(|k| k.contains(Key::BTN_LEFT)).unwrap_or(false);

            if !has_rel_x && !has_enter && !has_mouse_btn {
                continue;
            }

            info!("[continuity:input] grabbing device: {} ({})", name, path.display());

            if let Err(e) = device.grab() {
                warn!("[continuity:input] could not grab {}: {}", name, e);
                continue;
            }

            grabbed_any = true;
            let tx_c = tx.clone();
            let name_c = name.clone();
            
            let mut stream = device.into_event_stream()
                .map_err(|e| format!("failed to create event stream: {e}"))?;

            let task = tokio::spawn(async move {
                let mut esc_count = 0;
                let mut last_esc = Instant::now();

                while let Some(Ok(ev)) = stream.next().await {
                    match ev.event_type() {
                        EventType::RELATIVE => {
                            let axis = RelativeAxisType(ev.code());
                            if axis == RelativeAxisType::REL_X {
                                let _ = tx_c.send(InputEvent::CursorMove { dx: ev.value() as f64, dy: 0.0 }).await;
                            } else if axis == RelativeAxisType::REL_Y {
                                let _ = tx_c.send(InputEvent::CursorMove { dx: 0.0, dy: ev.value() as f64 }).await;
                            } else if axis == RelativeAxisType::REL_WHEEL {
                                let _ = tx_c.send(InputEvent::PointerAxis { dx: 0.0, dy: ev.value() as f64 }).await;
                            } else if axis == RelativeAxisType::REL_HWHEEL {
                                let _ = tx_c.send(InputEvent::PointerAxis { dx: ev.value() as f64, dy: 0.0 }).await;
                            }
                        }
                        EventType::KEY => {
                            let code = ev.code() as u32;
                            let val = ev.value();
                            
                            if code == Key::KEY_ESC.0 as u32 && val == 1 {
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
                                    break;
                                }
                            }

                            let is_mouse = code >= 272 && code <= 276;
                            if is_mouse {
                                if val == 1 {
                                    let _ = tx_c.send(InputEvent::PointerButton { button: code, state: 1 }).await;
                                } else if val == 0 {
                                    let _ = tx_c.send(InputEvent::PointerButton { button: code, state: 0 }).await;
                                }
                            } else {
                                if val == 1 || val == 2 {
                                    let _ = tx_c.send(InputEvent::KeyPress { key: code, state: 1 }).await;
                                } else if val == 0 {
                                    let _ = tx_c.send(InputEvent::KeyRelease { key: code }).await;
                                }
                            }
                        }
                        _ => {}
                    }
                }
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
