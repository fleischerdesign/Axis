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
