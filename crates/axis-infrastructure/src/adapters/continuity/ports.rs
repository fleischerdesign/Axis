use async_channel::Sender;
use async_trait::async_trait;
use axis_domain::models::continuity::{Message, Side};
use std::net::SocketAddr;

use super::clipboard::ClipboardEvent;
use super::connection::ConnectionEvent;
use super::discovery::DiscoveryEvent;
use super::input::InternalInputEvent;
use super::pipewire_devices::PipeWireAudioDevice;

pub trait ContinuityNetworkPort: Send + Sync {
    fn listen(&mut self, port: u16, tx: Sender<ConnectionEvent>) -> Result<(), String>;
    fn connect_dual(
        &mut self,
        addr_v4: SocketAddr,
        addr_v6: Option<SocketAddr>,
        tx: Sender<ConnectionEvent>,
        device_id: String,
        device_name: String,
    );
    fn disconnect_active(&mut self);
    fn stop(&mut self);
    fn send_message(&self, msg: Message);
    fn set_active_write(&mut self, write_tx: tokio::sync::mpsc::Sender<Message>);
    fn active_write_tx(&self) -> Option<tokio::sync::mpsc::Sender<Message>>;
}

#[async_trait]
pub trait ContinuityAudioPort: Send + Sync {
    async fn start_capture(
        &self,
        target_device: Option<&str>,
        tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    );
    async fn stop_capture(&self);
    async fn play_chunk(&self, target_device: Option<&str>, pcm_data: &[u8]);
    async fn stop_playback(&self);
    fn list_devices(&self) -> Vec<PipeWireAudioDevice>;
}

pub trait ContinuityCapturePort: Send + Sync {
    fn prepare(&mut self) -> Result<(), String>;
    fn start_capture(&mut self, tx: Sender<InternalInputEvent>) -> Result<(), String>;
    fn stop_capture(&mut self);
    fn is_capturing(&self) -> bool;
}

pub trait ContinuityInjectionPort: Send + Sync {
    fn start_injection(&mut self) -> Result<(), String>;
    fn stop_injection(&mut self);
    fn warp(
        &mut self,
        side: Side,
        edge_pos: f64,
        screen_w: i32,
        screen_h: i32,
    ) -> Result<(), String>;
    fn inject(&mut self, msg: &Message) -> Result<(), String>;
}

pub trait ContinuityClipboardPort: Send + Sync {
    fn start_monitoring(&mut self, tx: Sender<ClipboardEvent>) -> Result<(), String>;
    fn stop_monitoring(&mut self);
    fn set_content(&mut self, content: &[u8], mime_type: &str) -> Result<(), String>;
}

pub trait ContinuityDiscoveryPort: Send + Sync {
    fn register(&mut self, name: &str, port: u16) -> Result<(), String>;
    fn browse(&mut self, tx: Sender<DiscoveryEvent>) -> Result<(), String>;
    fn stop_browse(&mut self);
    fn stop(&mut self);
}
