use axis_domain::models::cloud::CloudStatus;
use axis_domain::ports::cloud::{CloudProvider, CloudError};
use std::sync::Arc;

pub struct GetCloudStatusUseCase {
    cloud_provider: Arc<dyn CloudProvider>,
}

impl GetCloudStatusUseCase {
    pub fn new(cloud_provider: Arc<dyn CloudProvider>) -> Self {
        Self { cloud_provider }
    }

    pub async fn execute(&self) -> Result<CloudStatus, CloudError> {
        self.cloud_provider.get_status().await
    }
}
