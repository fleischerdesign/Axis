use async_channel::Sender;
use log::{info, warn};
use evdev::uinput::VirtualDeviceBuilder;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Key, InputEvent as EvEvent, EventType, RelativeAxisType};

use super::protocol::Message;

// ── Input Events ───────────────────────────────────────────────────────

#[derive(Debug)]
pub enum InputEvent {
    CursorMove { dx: f64, dy: f64 },
    KeyPress { key: u32, state: u8 },
    KeyRelease { key: u32 },
    PointerButton { button: u32, state: u8 },
    PointerAxis { dx: f64, dy: f64 },
    EdgeHit { side: super::protocol::Side },
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

// ── evdev Implementation ───────────────────────────────────────────────

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
        
        // Setup keys (full keyboard support)
        let mut keys = AttributeSet::<Key>::new();
        for i in 0..512 {
            keys.insert(Key::new(i));
        }

        // Setup relative axes (mouse movement)
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
            .map_err(|e| format!("failed to build virtual device (check /dev/uinput permissions): {e}"))?;

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

// ── Stub Capture ──────────────────────────────────────────────────────

pub struct StubCapture {
    active: bool,
}

impl StubCapture {
    pub fn new() -> Self {
        Self { active: false }
    }
}

impl InputCapture for StubCapture {
    fn start(&mut self, _tx: Sender<InputEvent>) -> Result<(), String> {
        warn!("[continuity:input] stub capture — not yet implemented");
        self.active = true;
        Ok(())
    }

    fn stop(&mut self) {
        self.active = false;
    }

    fn is_capturing(&self) -> bool {
        self.active
    }
}
