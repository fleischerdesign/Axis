use async_trait::async_trait;
use axis_domain::models::config::AxisConfig;
use axis_domain::models::dnd::DndStatus;
use axis_domain::ports::config::ConfigProvider;
use axis_domain::ports::dnd::{DndError, DndProvider, DndStream};
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct ConfigDndProvider {
    config_provider: Arc<dyn ConfigProvider>,
    status_tx: watch::Sender<DndStatus>,
}

impl ConfigDndProvider {
    pub fn new(config_provider: Arc<dyn ConfigProvider>) -> Arc<Self> {
        let initial = Self::config_to_status(&config_provider.get());
        let (status_tx, _) = watch::channel(initial.clone());

        let provider = Arc::new(Self {
            config_provider: config_provider.clone(),
            status_tx,
        });

        let status_tx_bg = provider.status_tx.clone();
        let mut last = initial;
        tokio::spawn(async move {
            let mut stream = config_provider.subscribe();
            while let Some(config) = futures_util::StreamExt::next(&mut stream).await {
                let status = Self::config_to_status(&config);
                if status != last {
                    last = status.clone();
                    status_tx_bg.send_modify(|s| *s = status);
                }
            }
        });

        provider
    }

    fn config_to_status(config: &AxisConfig) -> DndStatus {
        DndStatus {
            enabled: config.dnd.enabled,
        }
    }
}

#[async_trait]
impl DndProvider for ConfigDndProvider {
    async fn get_status(&self) -> Result<DndStatus, DndError> {
        Ok(Self::config_to_status(&self.config_provider.get()))
    }

    async fn subscribe(&self) -> Result<DndStream, DndError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), DndError> {
        self.config_provider
            .update(Box::new(move |cfg| cfg.dnd.enabled = enabled));
        Ok(())
    }
}
