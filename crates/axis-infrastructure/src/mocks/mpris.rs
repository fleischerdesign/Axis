use axis_domain::models::mpris::{MprisStatus, MprisPlayer, PlaybackState};
use axis_domain::ports::mpris::{MprisProvider, MprisError, MprisStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;

pub struct MockMprisProvider {
    status_tx: watch::Sender<MprisStatus>,
}

impl MockMprisProvider {
    pub fn new() -> Arc<Self> {
        let (tx, _) = watch::channel(MprisStatus {
            players: vec![MprisPlayer {
                id: "mock".to_string(),
                title: "Mock Song".to_string(),
                artist: "Mock Artist".to_string(),
                album: "Mock Album".to_string(),
                art_url: None,
                playback: PlaybackState::Playing,
                position_us: 90_000_000,
                length_us: 240_000_000,
                can_play: true,
                can_pause: true,
                can_go_next: true,
                can_go_previous: true,
            }],
            active_player_id: Some("mock".to_string()),
        });
        Arc::new(Self { status_tx: tx })
    }
}

#[async_trait]
impl MprisProvider for MockMprisProvider {
    async fn get_status(&self) -> Result<MprisStatus, MprisError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<MprisStream, MprisError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn play_pause(&self, _player_id: &str) -> Result<(), MprisError> {
        let mut status = self.status_tx.borrow().clone();
        if let Some(player) = status.players.iter_mut().find(|p| p.id == _player_id) {
            player.playback = match player.playback {
                PlaybackState::Playing => PlaybackState::Paused,
                _ => PlaybackState::Playing,
            };
        }
        let _ = self.status_tx.send(status);
        Ok(())
    }

    async fn next(&self, _player_id: &str) -> Result<(), MprisError> {
        Ok(())
    }

    async fn previous(&self, _player_id: &str) -> Result<(), MprisError> {
        Ok(())
    }
}
