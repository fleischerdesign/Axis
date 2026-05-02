use async_trait::async_trait;
use axis_domain::models::config::AxisConfig;
use axis_domain::models::idle_inhibit::IdleInhibitStatus;
use axis_domain::ports::config::ConfigProvider;
use axis_domain::ports::idle_inhibit::{IdleInhibitError, IdleInhibitProvider, IdleInhibitStream};
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct ConfigIdleInhibitProvider {
    config_provider: Arc<dyn ConfigProvider>,
    status_tx: watch::Sender<IdleInhibitStatus>,
}

impl ConfigIdleInhibitProvider {
    pub async fn new(config_provider: Arc<dyn ConfigProvider>) -> Arc<Self> {
        let config = config_provider.get().unwrap_or_else(|e| {
            log::error!("[idle-inhibit] config get failed: {e}");
            AxisConfig::default()
        });
        let initial = Self::config_to_status(&config);
        let (status_tx, _) = watch::channel(initial.clone());

        let provider = Arc::new(Self {
            config_provider: config_provider.clone(),
            status_tx,
        });

        let status_tx_bg = provider.status_tx.clone();
        let mut last = initial;
        tokio::spawn(async move {
            let mut stream = match config_provider.subscribe() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("[idle-inhibit] config subscribe failed: {e}");
                    return;
                }
            };
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

    fn config_to_status(config: &AxisConfig) -> IdleInhibitStatus {
        IdleInhibitStatus {
            inhibited: config.idle_inhibit.enabled,
        }
    }
}

#[async_trait]
impl IdleInhibitProvider for ConfigIdleInhibitProvider {
    async fn get_status(&self) -> Result<IdleInhibitStatus, IdleInhibitError> {
        let config = self.config_provider.get()
            .map_err(|e| IdleInhibitError::ProviderError(e.to_string()))?;
        Ok(Self::config_to_status(&config))
    }

    async fn subscribe(&self) -> Result<IdleInhibitStream, IdleInhibitError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_inhibited(&self, inhibited: bool) -> Result<(), IdleInhibitError> {
        self.config_provider
            .update(Box::new(move |cfg| cfg.idle_inhibit.enabled = inhibited))
            .map_err(|e| IdleInhibitError::ProviderError(e.to_string()))
    }
}
