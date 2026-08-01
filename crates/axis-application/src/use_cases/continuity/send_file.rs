use axis_domain::ports::continuity::{ContinuityError, ContinuitySharingProvider};
use std::path::PathBuf;
use std::sync::Arc;

pub struct SendFileUseCase {
    provider: Arc<dyn ContinuitySharingProvider>,
}

impl SendFileUseCase {
    pub fn new(provider: Arc<dyn ContinuitySharingProvider>) -> Self {
        Self { provider }
    }

    pub async fn execute(&self, path: PathBuf, mime_type: String) -> Result<(), ContinuityError> {
        self.provider.send_file(path, mime_type).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axis_domain::models::continuity::{ContinuityStatus, InputEvent, Side};
    use std::sync::atomic::{AtomicBool, Ordering};

    struct TestSharingProvider {
        file_sent: AtomicBool,
    }

    #[async_trait]
    impl ContinuitySharingProvider for TestSharingProvider {
        async fn start_sharing(&self, _side: Side, _edge_pos: f64) -> Result<(), ContinuityError> {
            Ok(())
        }
        async fn stop_sharing(&self, _edge_pos: f64) -> Result<(), ContinuityError> {
            Ok(())
        }
        async fn send_input(&self, _event: InputEvent) -> Result<(), ContinuityError> {
            Ok(())
        }
        async fn force_local(&self) -> Result<(), ContinuityError> {
            Ok(())
        }
        async fn send_file(
            &self,
            _path: PathBuf,
            _mime_type: String,
        ) -> Result<(), ContinuityError> {
            self.file_sent.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_send_file_use_case() {
        let provider = Arc::new(TestSharingProvider {
            file_sent: AtomicBool::new(false),
        });
        let use_case = SendFileUseCase::new(provider.clone());
        let res = use_case
            .execute(PathBuf::from("/tmp/test.txt"), "text/plain".to_string())
            .await;
        assert!(res.is_ok());
        assert!(provider.file_sent.load(Ordering::SeqCst));
    }
}
