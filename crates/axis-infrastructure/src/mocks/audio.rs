use axis_domain::models::audio::{AudioStatus, AudioDevice, SinkInput};
use axis_domain::ports::audio::{AudioProvider, AudioError, AudioStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct MockAudioProvider {
    status_tx: watch::Sender<AudioStatus>,
}

impl MockAudioProvider {
    pub fn new() -> Arc<Self> {
        let (tx, _) = watch::channel(AudioStatus {
            volume: 0.5,
            is_muted: false,
            sink_inputs: vec![
                SinkInput {
                    id: 1,
                    name: "Music Player".to_string(),
                    volume: 0.8,
                }
            ],
            sinks: vec![
                AudioDevice {
                    id: 1,
                    name: "Built-in Speaker".to_string(),
                    description: "Internal Audio".to_string(),
                    is_default: true,
                },
                AudioDevice {
                    id: 2,
                    name: "HDMI Output".to_string(),
                    description: "HDMI Audio".to_string(),
                    is_default: false,
                },
            ],
            sources: vec![
                AudioDevice {
                    id: 1,
                    name: "Built-in Microphone".to_string(),
                    description: "Internal Audio".to_string(),
                    is_default: true,
                }
            ],
        });
        Arc::new(Self { status_tx: tx })
    }
}

#[async_trait]
impl AudioProvider for MockAudioProvider {
    async fn get_status(&self) -> Result<AudioStatus, AudioError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn set_volume(&self, volume: f64) -> Result<(), AudioError> {
        let mut status = self.status_tx.borrow().clone();
        status.volume = volume;
        let _ = self.status_tx.send(status);
        Ok(())
    }

    async fn set_muted(&self, muted: bool) -> Result<(), AudioError> {
        let mut status = self.status_tx.borrow().clone();
        status.is_muted = muted;
        let _ = self.status_tx.send(status);
        Ok(())
    }

    async fn subscribe(&self) -> Result<AudioStream, AudioError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_default_sink(&self, id: u32) -> Result<(), AudioError> {
        let mut status = self.status_tx.borrow().clone();
        for sink in status.sinks.iter_mut() {
            sink.is_default = sink.id == id;
        }
        let _ = self.status_tx.send(status);
        Ok(())
    }

    async fn set_default_source(&self, id: u32) -> Result<(), AudioError> {
        let mut status = self.status_tx.borrow().clone();
        for source in status.sources.iter_mut() {
            source.is_default = source.id == id;
        }
        let _ = self.status_tx.send(status);
        Ok(())
    }

    async fn set_sink_input_volume(&self, id: u32, volume: f64) -> Result<(), AudioError> {
        let mut status = self.status_tx.borrow().clone();
        if let Some(input) = status.sink_inputs.iter_mut().find(|i| i.id == id) {
            input.volume = volume;
        }
        let _ = self.status_tx.send(status);
        Ok(())
    }
}
