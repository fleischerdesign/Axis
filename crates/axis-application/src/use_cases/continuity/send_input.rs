use axis_domain::models::continuity::InputEvent;
use axis_domain::ports::continuity::{ContinuityProvider, ContinuityError};
use std::sync::Arc;

pub struct SendInputUseCase {
    provider: Arc<dyn ContinuityProvider>,
}

impl SendInputUseCase {
    pub fn new(provider: Arc<dyn ContinuityProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, event: InputEvent) -> Result<(), ContinuityError> {
        self.provider.send_input(event).await
    }
}
