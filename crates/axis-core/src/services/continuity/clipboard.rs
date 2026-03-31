use async_channel::Sender;
use log::{error, info};
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;

// ── Clipboard Events ──────────────────────────────────────────────────

#[derive(Debug)]
pub enum ClipboardEvent {
    ContentChanged { content: String, mime_type: String },
}

// ── Clipboard Provider Trait ──────────────────────────────────────────

pub trait ClipboardSync: Send {
    fn start_monitoring(&mut self, tx: Sender<ClipboardEvent>) -> Result<(), String>;
    fn stop_monitoring(&mut self);
    fn set_content(&mut self, content: &str, mime_type: &str) -> Result<(), String>;
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

    fn hash(content: &str) -> u64 {
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

        // We use wl-paste --watch to get notified on every change.
        // Since wl-paste --watch executes a command on change, we use a small trick:
        // We run it with 'sh -c "echo CHANGED"' and then we manually fetch the content.
        // Actually, a simpler way is to just run `wl-paste --watch cat`.
        let mut child = Command::new("wl-paste")
            .arg("--watch")
            .arg("cat")
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn wl-paste: {e}"))?;

        let stdout = child.stdout.take().ok_or("failed to take stdout")?;
        self.monitor_child = Some(child);

        let mut reader = BufReader::new(stdout);

        let task = tokio::spawn(async move {
            // Note: wl-paste cat will output the full clipboard content on every change.
            // This might be large, but for text/plain it's usually fine.
            // A more robust way would be using a specialized tool or raw Wayland protocols,
            // but wl-paste is the most reliable "just works" approach on NixOS/Niri.
            
            loop {
                // Read everything until EOF (which wl-paste cat provides for each change)
                let mut content = Vec::new();
                match reader.read_to_end(&mut content).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if let Ok(text) = String::from_utf8(content) {
                            let text = text.trim().to_string();
                            if !text.is_empty() {
                                let _ = tx.send(ClipboardEvent::ContentChanged {
                                    content: text,
                                    mime_type: "text/plain".to_string(),
                                }).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("[continuity:clipboard] monitor read error: {e}");
                        break;
                    }
                }
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

    fn set_content(&mut self, content: &str, mime_type: &str) -> Result<(), String> {
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
        let content_owned = content.to_string();
        
        tokio::spawn(async move {
            if let Err(e) = stdin.write_all(content_owned.as_bytes()).await {
                error!("[continuity:clipboard] wl-copy write error: {e}");
            }
            let _ = stdin.shutdown().await;
            let _ = child.wait().await;
        });

        Ok(())
    }
}
