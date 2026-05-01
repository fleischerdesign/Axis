use axis_domain::ports::notifications::{NotificationProvider, NotificationError};
use std::sync::Arc;
use log::info;

pub struct CloseNotificationUseCase {
    provider: Arc<dyn NotificationProvider>,
}

impl CloseNotificationUseCase {
    pub fn new(provider: Arc<dyn NotificationProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: u32) -> Result<(), NotificationError> {
        info!("[use-case] Closing notification {}", id);
        self.provider.close_notification(id).await
    }
}
