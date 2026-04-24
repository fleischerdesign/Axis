use axis_domain::models::notifications::NotificationStatus;
use axis_domain::ports::notifications::{NotificationService, NotificationError};
use std::sync::Arc;
use log::debug;

pub struct GetNotificationsUseCase {
    service: Arc<dyn NotificationService>,
}

impl GetNotificationsUseCase {
    pub fn new(service: Arc<dyn NotificationService>) -> Self {
        Self { service }
    }

    pub async fn execute(&self) -> Result<NotificationStatus, NotificationError> {
        debug!("[use-case] Fetching active notifications");
        self.service.get_status().await
    }
}
