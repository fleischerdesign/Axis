use axis_domain::ports::notifications::{NotificationProvider, NotificationError};
use std::sync::Arc;
use log::info;

pub struct InvokeNotificationActionUseCase {
    provider: Arc<dyn NotificationProvider>,
}

impl InvokeNotificationActionUseCase {
    pub fn new(provider: Arc<dyn NotificationProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: u32, action_key: &str, user_input: Option<String>) -> Result<(), NotificationError> {
        info!("[use-case] Invoking action '{}' on notification {}", action_key, id);
        self.provider.invoke_action(id, action_key, user_input).await
    }
}
