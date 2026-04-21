use axis_domain::ports::cloud::{CloudProvider, CloudError, CloudStream};
use std::sync::Arc;

pub struct SubscribeToCloudUpdatesUseCase {
    cloud_provider: Arc<dyn CloudProvider>,
}

impl SubscribeToCloudUpdatesUseCase {
    pub fn new(cloud_provider: Arc<dyn CloudProvider>) -> Self {
        Self { cloud_provider }
    }

    pub async fn execute(&self) -> Result<CloudStream, CloudError> {
        self.cloud_provider.subscribe().await
    }
}
