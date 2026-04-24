use axis_domain::ports::audio::{AudioProvider, AudioError};
use std::sync::Arc;
use log::{debug, info};

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

        if volume > 0.0 {
            if let Ok(status) = self.provider.get_status().await {
                if status.is_muted {
                    info!("[use-case] Auto-unmuting system");
                    let _ = self.provider.set_muted(false).await;
                }
            }
        }

        self.provider.set_volume(volume).await
    }
}
