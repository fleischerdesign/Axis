use axis_domain::models::notifications::Notification;
use axis_domain::ports::notifications::{ActionHandler, NotificationProvider, NotificationError};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ShowNotificationUseCase {
    provider: Arc<dyn NotificationProvider>,
}

impl ShowNotificationUseCase {
    pub fn new(provider: Arc<dyn NotificationProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(
        &self,
        notification: Notification,
        action_handlers: HashMap<String, ActionHandler>,
    ) -> Result<u32, NotificationError> {
        self.provider.show(notification, action_handlers).await
    }
}
