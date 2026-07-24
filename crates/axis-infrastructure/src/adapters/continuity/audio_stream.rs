use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use log::{error, info, warn};

async fn resolve_target(target: Option<&str>) -> Option<String> {
    match target {
        Some("@DEFAULT_SOURCE@") | None => target.map(|s| s.to_string()),
        Some(name) => {
            let output = Command::new("pw-cli")
                .args(["info", name])
                .output()
                .await
                .ok()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(id_str) = line.trim().strip_prefix("id: ") {
                    return Some(id_str.to_string());
                }
            }
            warn!("[continuity-audio] could not resolve PipeWire target '{name}', falling back to default source");
            None
        }
    }
}

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

        let resolved = resolve_target(target_device).await;

        let mut cmd = Command::new("pw-record");
        cmd.args([
            "--raw",
            "--format=s16",
            "--rate=44100",
            "--channels=2",
            "--latency=20ms",
        ]);
        if let Some(target) = resolved.as_deref()
            && !target.is_empty()
        {
            cmd.args(["--target", target]);
            info!("[continuity-audio] starting PipeWire audio capture via pw-record (target: {target})");
        } else {
            info!("[continuity-audio] starting PipeWire audio capture via pw-record (default source)");
        }
        cmd.arg("-");
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
                            match reader.read_exact(&mut buffer).await {
                                Ok(_) => {
                                    if tx.try_send(buffer.clone()).is_err() && tx.is_closed() {
                                        break;
                                    }
                                }

                                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                                    info!("[continuity-audio] capture stream ended (EOF)");
                                    break;
                                }
                                Err(e) => {
                                    error!("[continuity-audio] capture read_exact error: {e}");
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
            info!("[continuity-audio] starting PipeWire audio playback via pw-cat");

            let mut cmd = Command::new("pw-cat");
            cmd.args([
                "--playback",
                "--raw",
                "--format=s16",
                "--rate=44100",
                "--channels=2",
                "--latency=20ms",
            ]);
            if let Some(target) = target_device {
                cmd.args(["--target", target]);
            }
            cmd.arg("-");
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

#[async_trait::async_trait]
impl super::ports::ContinuityAudioPort for AudioStreamManager {
    async fn start_capture(
        &self,
        target_device: Option<&str>,
        tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) {
        AudioStreamManager::start_capture(self, target_device, tx).await;
    }

    async fn stop_capture(&self) {
        AudioStreamManager::stop_capture(self).await;
    }

    async fn play_chunk(&self, target_device: Option<&str>, pcm_data: &[u8]) {
        AudioStreamManager::play_chunk(self, target_device, pcm_data).await;
    }

    async fn stop_playback(&self) {
        AudioStreamManager::stop_playback(self).await;
    }

    fn list_devices(&self) -> Vec<super::pipewire_devices::PipeWireAudioDevice> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(super::pipewire_devices::list_pipewire_audio_devices())
    }
}
