use axis_domain::models::continuity::InputEvent;
use axis_domain::ports::continuity::{ContinuityError, ContinuitySharingProvider};
use log::info;
use std::sync::Arc;

pub struct SendInputUseCase {
    provider: Arc<dyn ContinuitySharingProvider>,
}

impl SendInputUseCase {
    pub fn new(provider: Arc<dyn ContinuitySharingProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, event: InputEvent) -> Result<(), ContinuityError> {
        info!("[use-case] Sending input");
        self.provider.send_input(event).await
    }
}
