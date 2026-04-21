use axis_domain::ports::network::{NetworkProvider, NetworkError};
use std::sync::Arc;

pub struct ConnectToApUseCase {
    provider: Arc<dyn NetworkProvider>,
}

impl ConnectToApUseCase {
    pub fn new(provider: Arc<dyn NetworkProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, id: &str, password: Option<&str>) -> Result<(), NetworkError> {
        self.provider.connect_to_ap(id, password).await
    }
}
