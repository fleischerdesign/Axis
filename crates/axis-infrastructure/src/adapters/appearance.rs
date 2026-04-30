use async_trait::async_trait;
use axis_domain::models::appearance::{AccentColor, ColorScheme};
use axis_domain::models::config::{AppearanceConfig, AxisConfig};
use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider, AppearanceStream};
use axis_domain::ports::config::ConfigProvider;
use log::info;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct ConfigAppearanceProvider {
    config_provider: Arc<dyn ConfigProvider>,
    status_tx: watch::Sender<AppearanceConfig>,
}

impl ConfigAppearanceProvider {
    pub async fn new(config_provider: Arc<dyn ConfigProvider>) -> Arc<Self> {
        let config = config_provider.get().unwrap_or_else(|e| {
            log::error!("[appearance] config get failed: {e}");
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
                    log::warn!("[appearance] config subscribe failed: {e}");
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

    fn config_to_status(config: &AxisConfig) -> AppearanceConfig {
        config.appearance.clone()
    }
}

#[async_trait]
impl AppearanceProvider for ConfigAppearanceProvider {
    async fn get_status(&self) -> Result<AppearanceConfig, AppearanceError> {
        let config = self.config_provider.get()
            .map_err(|e| AppearanceError::ProviderError(e.to_string()))?;
        Ok(Self::config_to_status(&config))
    }

    async fn subscribe(&self) -> Result<AppearanceStream, AppearanceError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_wallpaper(&self, path: Option<String>) -> Result<(), AppearanceError> {
        info!("[appearance] Setting wallpaper: {:?}", path);
        self.config_provider
            .update(Box::new(move |cfg| cfg.appearance.wallpaper = path))
            .map_err(|e| AppearanceError::ProviderError(e.to_string()))
    }

    async fn set_accent_color(&self, color: AccentColor) -> Result<(), AppearanceError> {
        info!("[appearance] Setting accent color: {:?}", color);
        self.config_provider
            .update(Box::new(move |cfg| cfg.appearance.accent_color = color))
            .map_err(|e| AppearanceError::ProviderError(e.to_string()))
    }

    async fn set_color_scheme(&self, scheme: ColorScheme) -> Result<(), AppearanceError> {
        info!("[appearance] Setting color scheme: {:?}", scheme);
        self.config_provider
            .update(Box::new(move |cfg| cfg.appearance.color_scheme = scheme))
            .map_err(|e| AppearanceError::ProviderError(e.to_string()))
    }

    async fn set_font(&self, font: Option<String>) -> Result<(), AppearanceError> {
        info!("[appearance] Setting font: {:?}", font);
        self.config_provider
            .update(Box::new(move |cfg| cfg.appearance.font = font))
            .map_err(|e| AppearanceError::ProviderError(e.to_string()))
    }
}
