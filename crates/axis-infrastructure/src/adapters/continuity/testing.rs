use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use async_channel::Sender;
use async_trait::async_trait;
use axis_domain::models::continuity::{Message, PeerInfo, Side};

use super::clipboard::ClipboardEvent;
use super::connection::ConnectionEvent;
use super::discovery::DiscoveryEvent;
use super::input::InternalInputEvent;
use super::pipewire_devices::PipeWireAudioDevice;
use super::ports::{
    ContinuityAudioPort, ContinuityCapturePort, ContinuityClipboardPort,
    ContinuityDiscoveryPort, ContinuityInjectionPort, ContinuityNetworkPort,
};

#[derive(Clone)]
pub struct MockNetwork {
    pub sent: Arc<Mutex<Vec<Message>>>,
    pub active_write: Arc<Mutex<Option<tokio::sync::mpsc::Sender<Message>>>>,
}

impl MockNetwork {
    pub fn new() -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
            active_write: Arc::new(Mutex::new(None)),
        }
    }

    pub fn drain_sent(&self) -> Vec<Message> {
        self.sent.lock().unwrap().drain(..).collect()
    }
}

impl ContinuityNetworkPort for MockNetwork {
    fn listen(&mut self, _port: u16, _tx: Sender<ConnectionEvent>) -> Result<(), String> {
        Ok(())
    }

    fn connect_dual(
        &mut self,
        _addr_v4: SocketAddr,
        _addr_v6: Option<SocketAddr>,
        _tx: Sender<ConnectionEvent>,
        _device_id: String,
        _device_name: String,
    ) {
    }

    fn disconnect_active(&mut self) {
        *self.active_write.lock().unwrap() = None;
    }

    fn stop(&mut self) {
        *self.active_write.lock().unwrap() = None;
    }

    fn send_message(&self, msg: Message) {
        self.sent.lock().unwrap().push(msg);
    }

    fn set_active_write(&mut self, write_tx: tokio::sync::mpsc::Sender<Message>) {
        *self.active_write.lock().unwrap() = Some(write_tx);
    }

    fn active_write_tx(&self) -> Option<tokio::sync::mpsc::Sender<Message>> {
        self.active_write.lock().unwrap().clone()
    }
}

#[derive(Clone)]
pub struct MockAudio {
    pub capture_targets: Arc<Mutex<Vec<Option<String>>>>,
    pub played_chunks: Arc<Mutex<Vec<Vec<u8>>>>,
    pub capture_active: Arc<Mutex<bool>>,
    pub playback_active: Arc<Mutex<bool>>,
}

impl MockAudio {
    pub fn new() -> Self {
        Self {
            capture_targets: Arc::new(Mutex::new(Vec::new())),
            played_chunks: Arc::new(Mutex::new(Vec::new())),
            capture_active: Arc::new(Mutex::new(false)),
            playback_active: Arc::new(Mutex::new(false)),
        }
    }

    pub fn drain_played(&self) -> Vec<Vec<u8>> {
        self.played_chunks.lock().unwrap().drain(..).collect()
    }
}

#[async_trait]
impl ContinuityAudioPort for MockAudio {
    async fn start_capture(
        &self,
        target_device: Option<&str>,
        _tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    ) {
        self.capture_targets
            .lock()
            .unwrap()
            .push(target_device.map(|s| s.to_string()));
        *self.capture_active.lock().unwrap() = true;
    }

    async fn stop_capture(&self) {
        *self.capture_active.lock().unwrap() = false;
    }

    async fn play_chunk(&self, _target_device: Option<&str>, pcm_data: &[u8]) {
        self.played_chunks
            .lock()
            .unwrap()
            .push(pcm_data.to_vec());
        *self.playback_active.lock().unwrap() = true;
    }

    async fn stop_playback(&self) {
        *self.playback_active.lock().unwrap() = false;
    }

    fn list_devices(&self) -> Vec<PipeWireAudioDevice> {
        vec![PipeWireAudioDevice {
            id: "@DEFAULT_MONITOR@".to_string(),
            name: "@DEFAULT_MONITOR@".to_string(),
            description: "System Sound".to_string(),
            is_sink_monitor: true,
            is_source: false,
        }]
    }
}

#[derive(Clone)]
pub struct MockCapture {
    pub prepared: Arc<Mutex<bool>>,
    pub events: Arc<Mutex<Vec<InternalInputEvent>>>,
    pub capturing: Arc<Mutex<bool>>,
}

impl MockCapture {
    pub fn new() -> Self {
        Self {
            prepared: Arc::new(Mutex::new(false)),
            events: Arc::new(Mutex::new(Vec::new())),
            capturing: Arc::new(Mutex::new(false)),
        }
    }
}

impl ContinuityCapturePort for MockCapture {
    fn prepare(&mut self) -> Result<(), String> {
        *self.prepared.lock().unwrap() = true;
        Ok(())
    }

    fn start_capture(&mut self, _tx: Sender<InternalInputEvent>) -> Result<(), String> {
        *self.capturing.lock().unwrap() = true;
        Ok(())
    }

    fn stop_capture(&mut self) {
        *self.capturing.lock().unwrap() = false;
    }

    fn is_capturing(&self) -> bool {
        *self.capturing.lock().unwrap()
    }
}

#[derive(Clone)]
pub struct MockInjection {
    pub started: Arc<Mutex<bool>>,
    pub injected: Arc<Mutex<Vec<Message>>>,
    pub warps: Arc<Mutex<Vec<(Side, f64, i32, i32)>>>,
}

impl MockInjection {
    pub fn new() -> Self {
        Self {
            started: Arc::new(Mutex::new(false)),
            injected: Arc::new(Mutex::new(Vec::new())),
            warps: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ContinuityInjectionPort for MockInjection {
    fn start_injection(&mut self) -> Result<(), String> {
        *self.started.lock().unwrap() = true;
        Ok(())
    }

    fn stop_injection(&mut self) {
        *self.started.lock().unwrap() = false;
    }

    fn warp(
        &mut self,
        side: Side,
        edge_pos: f64,
        screen_w: i32,
        screen_h: i32,
    ) -> Result<(), String> {
        self.warps
            .lock()
            .unwrap()
            .push((side, edge_pos, screen_w, screen_h));
        Ok(())
    }

    fn inject(&mut self, msg: &Message) -> Result<(), String> {
        self.injected.lock().unwrap().push(msg.clone());
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockClipboard {
    pub content: Arc<Mutex<Vec<u8>>>,
    pub mime_type: Arc<Mutex<String>>,
    pub monitoring: Arc<Mutex<bool>>,
}

impl MockClipboard {
    pub fn new() -> Self {
        Self {
            content: Arc::new(Mutex::new(Vec::new())),
            mime_type: Arc::new(Mutex::new(String::new())),
            monitoring: Arc::new(Mutex::new(false)),
        }
    }
}

impl ContinuityClipboardPort for MockClipboard {
    fn start_monitoring(&mut self, _tx: Sender<ClipboardEvent>) -> Result<(), String> {
        *self.monitoring.lock().unwrap() = true;
        Ok(())
    }

    fn stop_monitoring(&mut self) {
        *self.monitoring.lock().unwrap() = false;
    }

    fn set_content(&mut self, content: &[u8], mime_type: &str) -> Result<(), String> {
        *self.content.lock().unwrap() = content.to_vec();
        *self.mime_type.lock().unwrap() = mime_type.to_string();
        Ok(())
    }
}

#[derive(Clone)]
pub struct MockDiscovery {
    pub registered: Arc<Mutex<bool>>,
    pub browsing: Arc<Mutex<bool>>,
    pub peers: Arc<Mutex<Vec<PeerInfo>>>,
}

impl MockDiscovery {
    pub fn new() -> Self {
        Self {
            registered: Arc::new(Mutex::new(false)),
            browsing: Arc::new(Mutex::new(false)),
            peers: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ContinuityDiscoveryPort for MockDiscovery {
    fn register(&mut self, _name: &str, _port: u16) -> Result<(), String> {
        *self.registered.lock().unwrap() = true;
        Ok(())
    }

    fn browse(&mut self, _tx: Sender<DiscoveryEvent>) -> Result<(), String> {
        *self.browsing.lock().unwrap() = true;
        Ok(())
    }

    fn stop_browse(&mut self) {
        *self.browsing.lock().unwrap() = false;
    }

    fn stop(&mut self) {
        *self.registered.lock().unwrap() = false;
        *self.browsing.lock().unwrap() = false;
    }
}
