use axis_domain::models::notifications::NotificationStatus;
use axis_domain::ports::notifications::{NotificationService, NotificationError};
use std::sync::Arc;

pub struct GetNotificationsUseCase {
    service: Arc<dyn NotificationService>,
}

impl GetNotificationsUseCase {
    pub fn new(service: Arc<dyn NotificationService>) -> Self {
        Self { service }
    }

    pub async fn execute(&self) -> Result<NotificationStatus, NotificationError> {
        self.service.get_status().await
    }
}
