use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use log::{error, info, warn};

pub struct AudioStreamManager {
    record_child: Arc<Mutex<Option<Child>>>,
    play_child: Arc<Mutex<Option<Child>>>,
    play_stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
}

impl Default for AudioStreamManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStreamManager {
    pub fn new() -> Self {
        Self {
            record_child: Arc::new(Mutex::new(None)),
            play_child: Arc::new(Mutex::new(None)),
            play_stdin: Arc::new(Mutex::new(None)),
        }
    }

    /// Starts capturing PCM audio from local PipeWire output/microphone and streams chunks to `tx`.
    pub async fn start_capture(
        &self,
        target_device: Option<&str>,
        tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) {
        self.stop_capture().await;

        let target = target_device.unwrap_or("@DEFAULT_MONITOR@");
        info!("[continuity-audio] starting PipeWire audio capture via pw-record (target: {target})");

        let mut cmd = Command::new("pw-record");
        cmd.args([
            "--target",
            target,
            "--format=s16",
            "--rate=44100",
            "--channels=2",
            "--latency=20ms",
            "-",
        ]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::null());

        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(stdout) = child.stdout.take() {
                    let mut lock = self.record_child.lock().await;
                    *lock = Some(child);

                    tokio::spawn(async move {
                        let mut reader = stdout;
                        // 20ms chunk size for 44.1kHz 16-bit 2-channel PCM = 44100 * 2 * 2 * 0.020 = 3528 bytes
                        let mut buffer = vec![0u8; 3528];
                        loop {
                            match reader.read(&mut buffer).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    if tx.send(buffer[..n].to_vec()).await.is_err() {
                                        break; // Receiver disconnected
                                    }
                                }
                                Err(e) => {
                                    error!("[continuity-audio] capture read error: {e}");
                                    break;
                                }
                            }
                        }
                    });
                }
            }
            Err(e) => {
                error!("[continuity-audio] failed to spawn pw-record: {e}");
            }
        }
    }

    pub async fn stop_capture(&self) {
        let mut lock = self.record_child.lock().await;
        if let Some(mut child) = lock.take() {
            let _ = child.start_kill();
            info!("[continuity-audio] stopped PipeWire audio capture");
        }
    }

    /// Plays an incoming PCM audio chunk over PipeWire speakers using `pw-cat`.
    pub async fn play_chunk(&self, target_device: Option<&str>, pcm_data: &[u8]) {
        let mut stdin_lock = self.play_stdin.lock().await;
        if stdin_lock.is_none() {
            let target = target_device.unwrap_or("@DEFAULT_SINK@");
            info!("[continuity-audio] starting PipeWire audio playback via pw-cat (target: {target})");

            let mut cmd = Command::new("pw-cat");
            cmd.args([
                "--playback",
                "--target",
                target,
                "--format=s16",
                "--rate=44100",
                "--channels=2",
                "--latency=20ms",
                "-",
            ]);
            cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());

            match cmd.spawn() {
                Ok(mut child) => {
                    if let Some(stdin) = child.stdin.take() {
                        *stdin_lock = Some(stdin);
                        let mut child_lock = self.play_child.lock().await;
                        *child_lock = Some(child);
                    }
                }
                Err(e) => {
                    error!("[continuity-audio] failed to spawn pw-cat: {e}");
                    return;
                }
            }
        }

        if let Some(stdin) = stdin_lock.as_mut()
            && let Err(e) = stdin.write_all(pcm_data).await
        {
            warn!("[continuity-audio] playback write error: {e}");
            *stdin_lock = None; // Reset on pipe error so it re-spawns next chunk
        }
    }

    pub async fn stop_playback(&self) {
        let mut stdin_lock = self.play_stdin.lock().await;
        *stdin_lock = None;
        let mut child_lock = self.play_child.lock().await;
        if let Some(mut child) = child_lock.take() {
            let _ = child.start_kill();
            info!("[continuity-audio] stopped PipeWire audio playback");
        }
    }
}
