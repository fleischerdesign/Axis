use async_channel::Sender;
use log::{info, warn};

// ── Clipboard Events ──────────────────────────────────────────────────

#[derive(Debug)]
pub enum ClipboardEvent {
    ContentChanged {
        content: String,
        mime_type: String,
    },
}

// ── Clipboard Provider Trait ───────────────────────────────────────────

pub trait ClipboardSync: Send {
    fn start_monitoring(&mut self, tx: Sender<ClipboardEvent>) -> Result<(), String>;
    fn stop_monitoring(&mut self);
    fn set_content(&self, content: &str, mime_type: &str) -> Result<(), String>;
}

// ── Stub Implementation ────────────────────────────────────────────────
// TODO: Implement with wl-paste --watch / wl-copy subprocesses.
// wl-clipboard-rs does not support clipboard monitoring (Issue #5).
// Approach:
// - Monitoring: spawn `wl-paste --watch --type text/plain` as subprocess,
//   read stdout for changes, send ClipboardEvent
// - Setting: call `wl-copy` with content via stdin pipe
// - Dedup: track last sent hash to avoid feedback loops

pub struct StubClipboard {
    monitoring: bool,
    last_hash: u64,
}

impl StubClipboard {
    pub fn new() -> Self {
        Self {
            monitoring: false,
            last_hash: 0,
        }
    }

    fn hash(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

impl ClipboardSync for StubClipboard {
    fn start_monitoring(&mut self, _tx: Sender<ClipboardEvent>) -> Result<(), String> {
        warn!("[continuity:clipboard] stub monitor — not yet implemented");
        self.monitoring = true;
        Ok(())
    }

    fn stop_monitoring(&mut self) {
        self.monitoring = false;
    }

    fn set_content(&self, content: &str, mime_type: &str) -> Result<(), String> {
        let hash = Self::hash(content);
        if hash == self.last_hash {
            return Ok(()); // Dedup: skip our own content
        }
        info!(
            "[continuity:clipboard] stub set_content: {} bytes ({mime_type})",
            content.len()
        );
        Ok(())
    }
}
