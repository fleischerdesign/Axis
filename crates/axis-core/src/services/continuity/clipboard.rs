use async_channel::Sender;
use log::{error, info, warn};
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;

const MAX_CLIPBOARD_SIZE: usize = 10 * 1024 * 1024;

// ── Clipboard Events ──────────────────────────────────────────────────

#[derive(Debug)]
pub enum ClipboardEvent {
    ContentChanged { content: Vec<u8>, mime_type: String },
}

// ── Clipboard Provider Trait ──────────────────────────────────────────

pub trait ClipboardSync: Send {
    fn start_monitoring(&mut self, tx: Sender<ClipboardEvent>) -> Result<(), String>;
    fn stop_monitoring(&mut self);
    fn set_content(&mut self, content: &[u8], mime_type: &str) -> Result<(), String>;
}

// ── Wayland Implementation ─────────────────────────────────────────────

pub struct WaylandClipboard {
    monitor_task: Option<JoinHandle<()>>,
    monitor_child: Option<Child>,
    last_hash: u64,
}

impl WaylandClipboard {
    pub fn new() -> Self {
        Self {
            monitor_task: None,
            monitor_child: None,
            last_hash: 0,
        }
    }

    fn hash(content: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

impl ClipboardSync for WaylandClipboard {
    fn start_monitoring(&mut self, tx: Sender<ClipboardEvent>) -> Result<(), String> {
        self.stop_monitoring();

        info!("[continuity:clipboard] starting monitoring via wl-paste");

        let mut child = Command::new("wl-paste")
            .arg("--watch")
            .arg("cat")
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn wl-paste: {e}"))?;

        let stdout = child.stdout.take().ok_or("failed to take stdout")?;
        self.monitor_child = Some(child);

        let mut reader = stdout;

        let task = tokio::spawn(async move {
            loop {
                let mut content = Vec::new();
                let mut remaining = MAX_CLIPBOARD_SIZE;
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            let take = n.min(remaining);
                            content.extend_from_slice(&buf[..take]);
                            remaining = remaining.saturating_sub(take);
                            if remaining == 0 {
                                warn!("[continuity:clipboard] content exceeded {} byte limit, truncated", MAX_CLIPBOARD_SIZE);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("[continuity:clipboard] monitor read error: {e}");
                            break;
                        }
                    }
                }
                if content.is_empty() {
                    break;
                }
                let _ = tx.send(ClipboardEvent::ContentChanged {
                    content,
                    mime_type: "text/plain".to_string(),
                }).await;
            }
        });

        self.monitor_task = Some(task);
        Ok(())
    }

    fn stop_monitoring(&mut self) {
        if let Some(mut child) = self.monitor_child.take() {
            let _ = child.kill();
        }
        if let Some(task) = self.monitor_task.take() {
            task.abort();
        }
    }

    fn set_content(&mut self, content: &[u8], mime_type: &str) -> Result<(), String> {
        let hash = Self::hash(content);
        if hash == self.last_hash {
            return Ok(()); // Dedup: avoid loops
        }
        self.last_hash = hash;

        info!(
            "[continuity:clipboard] setting content: {} bytes ({mime_type})",
            content.len()
        );

        // Use wl-copy to set the clipboard
        let mut child = Command::new("wl-copy")
            .arg("--type")
            .arg(mime_type)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn wl-copy: {e}"))?;

        let mut stdin = child.stdin.take().ok_or("failed to take stdin")?;
        let content_owned = content.to_vec();
        
        tokio::spawn(async move {
            if let Err(e) = stdin.write_all(&content_owned).await {
                error!("[continuity:clipboard] wl-copy write error: {e}");
            }
            let _ = stdin.shutdown().await;
            let _ = child.wait().await;
        });

        Ok(())
    }
}
