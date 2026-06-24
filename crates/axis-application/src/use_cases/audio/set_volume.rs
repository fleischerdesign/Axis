use axis_domain::ports::audio::{AudioError, AudioProvider};
use log::{debug, info};
use std::sync::Arc;

pub struct SetVolumeUseCase {
    provider: Arc<dyn AudioProvider>,
}

impl SetVolumeUseCase {
    pub fn new(provider: Arc<dyn AudioProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, volume: f64) -> Result<(), AudioError> {
        let volume = volume.clamp(0.0, 1.5);
        debug!("[use-case] Setting system volume to {:.0}%", volume * 100.0);

        if volume > 0.0
            && let Ok(status) = self.provider.get_status().await
            && status.is_muted
        {
            info!("[use-case] Auto-unmuting system");
            if let Err(e) = self.provider.set_muted(false).await {
                log::warn!("[use-case] Auto-unmute failed: {e}");
            }
        }

        self.provider.set_volume(volume).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axis_infrastructure::mocks::audio::MockAudioProvider;

    #[tokio::test]
    async fn set_volume_normal() {
        let mock = MockAudioProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetVolumeUseCase::new(mock.clone());
        uc.execute(0.75).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert_eq!(status.volume, 0.75);
        assert!(!status.is_muted);
    }

    #[tokio::test]
    async fn set_volume_clamps_to_max() {
        let mock = MockAudioProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        let uc = SetVolumeUseCase::new(mock.clone());
        uc.execute(2.0).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert_eq!(status.volume, 1.5);
    }

    #[tokio::test]
    async fn set_volume_auto_unmutes() {
        let mock = MockAudioProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        mock.set_muted(true).await.unwrap();
        let uc = SetVolumeUseCase::new(mock.clone());
        uc.execute(0.5).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!(!status.is_muted);
        assert_eq!(status.volume, 0.5);
    }

    #[tokio::test]
    async fn set_volume_zero_does_not_unmute() {
        let mock = MockAudioProvider::new();
        let _rx = mock.subscribe().await.unwrap();
        mock.set_muted(true).await.unwrap();
        let uc = SetVolumeUseCase::new(mock.clone());
        uc.execute(0.0).await.unwrap();
        let status = mock.get_status().await.unwrap();
        assert!(status.is_muted);
    }
}
