use axis_domain::models::continuity::InputEvent;
use axis_domain::ports::continuity::{ContinuitySharingProvider, ContinuityError};
use std::sync::Arc;
use log::info;

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
