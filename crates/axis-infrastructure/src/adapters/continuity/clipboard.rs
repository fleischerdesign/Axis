use async_channel::Sender;
use log::{error, info, warn};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;

const MAX_CLIPBOARD_SIZE: usize = 10 * 1024 * 1024;

// --- Clipboard Events --------------------------------------------------

#[derive(Debug)]
pub enum ClipboardEvent {
    ContentChanged { content: Vec<u8>, mime_type: String },
}

// --- Clipboard Provider Trait ------------------------------------------

pub trait ClipboardSync: Send {
    fn start_monitoring(&mut self, tx: Sender<ClipboardEvent>) -> Result<(), String>;
    fn stop_monitoring(&mut self);
    fn set_content(&mut self, content: &[u8], mime_type: &str) -> Result<(), String>;
}

// --- Wayland Implementation ---------------------------------------------

pub struct WaylandClipboard {
    monitor_task: Option<JoinHandle<()>>,
    monitor_child: Option<Child>,
    last_hash: Arc<AtomicU64>,
}

impl Default for WaylandClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl WaylandClipboard {
    pub fn new() -> Self {
        Self {
            monitor_task: None,
            monitor_child: None,
            last_hash: Arc::new(AtomicU64::new(0)),
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
            .arg("sh")
            .arg("-c")
            .arg("cat; printf \"\\n---AXIS_CLIP_END---\\n\"")
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn wl-paste: {e}"))?;

        let stdout = child.stdout.take().ok_or("failed to take stdout")?;
        self.monitor_child = Some(child);

        let mut reader = tokio::io::BufReader::new(stdout);
        let last_hash = self.last_hash.clone();

        let task = tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut accumulator: Vec<u8> = Vec::new();
            let mut line_buf = String::new();

            loop {
                line_buf.clear();
                match reader.read_line(&mut line_buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        if line_buf.trim_end_matches(['\r', '\n']) == "---AXIS_CLIP_END---" {
                            if accumulator.ends_with(b"\n") {
                                accumulator.pop();
                                if accumulator.ends_with(b"\r") {
                                    accumulator.pop();
                                }
                            }
                            if !accumulator.is_empty() {
                                let hash = Self::hash(&accumulator);
                                if hash != last_hash.load(Ordering::Relaxed) {
                                    last_hash.store(hash, Ordering::Relaxed);
                                    info!(
                                        "[continuity:clipboard] detected clipboard change: {} bytes",
                                        accumulator.len()
                                    );
                                    let _ = tx
                                        .send(ClipboardEvent::ContentChanged {
                                            content: std::mem::take(&mut accumulator),
                                            mime_type: "text/plain".to_string(),
                                        })
                                        .await;
                                } else {
                                    accumulator.clear();
                                }
                            }
                        } else {
                            if accumulator.len() + line_buf.len() <= MAX_CLIPBOARD_SIZE {
                                accumulator.extend_from_slice(line_buf.as_bytes());
                            } else {
                                warn!(
                                    "[continuity:clipboard] content exceeded {} byte limit, truncated",
                                    MAX_CLIPBOARD_SIZE
                                );
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
            let _ = child.start_kill();
        }
        if let Some(task) = self.monitor_task.take() {
            task.abort();
        }
    }

    fn set_content(&mut self, content: &[u8], mime_type: &str) -> Result<(), String> {
        let hash = Self::hash(content);
        if hash == self.last_hash.load(Ordering::Relaxed) {
            return Ok(()); // Dedup: avoid loops
        }
        self.last_hash.store(hash, Ordering::Relaxed);

        info!(
            "[continuity:clipboard] setting content: {} bytes ({mime_type})",
            content.len()
        );

        // Use wl-copy via sh -c to detach the background daemon process cleanly
        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        if mime_type != "text/plain" && !mime_type.is_empty() {
            cmd.arg(format!("wl-copy --type '{mime_type}'"));
        } else {
            cmd.arg("tee >(wl-copy) | wl-copy --primary");
        }

        let mut child = cmd
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn wl-copy: {e}"))?;

        let mut stdin = child.stdin.take().ok_or("failed to take stdin")?;
        let stderr = child.stderr.take();
        let content_owned = content.to_vec();

        tokio::spawn(async move {
            if let Err(e) = stdin.write_all(&content_owned).await {
                error!("[continuity:clipboard] wl-copy write error: {e}");
            }
            let _ = stdin.shutdown().await;

            if let Some(mut err_stream) = stderr {
                use tokio::io::AsyncReadExt;
                let mut err_buf = Vec::new();
                let _ = err_stream.read_to_end(&mut err_buf).await;
                if !err_buf.is_empty() {
                    let err_msg = String::from_utf8_lossy(&err_buf);
                    error!("[continuity:clipboard] wl-copy stderr: {}", err_msg.trim());
                }
            }

            match child.wait().await {
                Ok(status) => {
                    if !status.success() {
                        error!("[continuity:clipboard] wl-copy exited with status: {status}");
                    } else {
                        info!("[continuity:clipboard] wl-copy completed successfully");
                    }
                }
                Err(e) => {
                    error!("[continuity:clipboard] wl-copy wait error: {e}");
                }
            }
        });

        Ok(())
    }
}
