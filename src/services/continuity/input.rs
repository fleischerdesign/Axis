use async_channel::Sender;
use log::{info, warn};

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

// ── Stub Implementation ────────────────────────────────────────────────
// TODO: Implement with reis (libei) crate for actual Wayland input
// capture and injection. Requires:
// - reis crate with tokio feature
// - Input Capture portal support from compositor (niri)
// - Fallback: evdev-based capture (requires root/udev)

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

pub struct StubInjection {
    active: bool,
}

impl StubInjection {
    pub fn new() -> Self {
        Self { active: false }
    }
}

impl InputInjection for StubInjection {
    fn start(&mut self) -> Result<(), String> {
        warn!("[continuity:input] stub injection — not yet implemented");
        self.active = true;
        Ok(())
    }

    fn stop(&mut self) {
        self.active = false;
    }

    fn inject(&mut self, msg: &Message) -> Result<(), String> {
        if !self.active {
            return Err("injection not active".into());
        }
        info!("[continuity:input] stub inject: {:?}", msg);
        Ok(())
    }
}
