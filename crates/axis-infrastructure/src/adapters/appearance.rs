use async_trait::async_trait;
use axis_domain::models::appearance::{AccentColor, AppearanceStatus, ColorScheme};
use axis_domain::models::config::AxisConfig;
use axis_domain::ports::appearance::{AppearanceError, AppearanceProvider, AppearanceStream};
use axis_domain::ports::config::ConfigProvider;
use log::info;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct ConfigAppearanceProvider {
    config_provider: Arc<dyn ConfigProvider>,
    status_tx: watch::Sender<AppearanceStatus>,
}

impl ConfigAppearanceProvider {
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

    fn config_to_status(config: &AxisConfig) -> AppearanceStatus {
        AppearanceStatus {
            wallpaper: config.appearance.wallpaper.clone(),
            accent_color: config.appearance.accent_color.clone(),
            color_scheme: config.appearance.color_scheme.clone(),
            font: config.appearance.font.clone(),
        }
    }
}

#[async_trait]
impl AppearanceProvider for ConfigAppearanceProvider {
    async fn get_status(&self) -> Result<AppearanceStatus, AppearanceError> {
        Ok(Self::config_to_status(&self.config_provider.get()))
    }

    async fn subscribe(&self) -> Result<AppearanceStream, AppearanceError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_wallpaper(&self, path: Option<String>) -> Result<(), AppearanceError> {
        info!("[appearance] Setting wallpaper: {:?}", path);
        self.config_provider
            .update(Box::new(move |cfg| cfg.appearance.wallpaper = path));
        Ok(())
    }

    async fn set_accent_color(&self, color: AccentColor) -> Result<(), AppearanceError> {
        info!("[appearance] Setting accent color: {:?}", color);
        self.config_provider
            .update(Box::new(move |cfg| cfg.appearance.accent_color = color));
        Ok(())
    }

    async fn set_color_scheme(&self, scheme: ColorScheme) -> Result<(), AppearanceError> {
        info!("[appearance] Setting color scheme: {:?}", scheme);
        self.config_provider
            .update(Box::new(move |cfg| cfg.appearance.color_scheme = scheme));
        Ok(())
    }

    async fn set_font(&self, font: Option<String>) -> Result<(), AppearanceError> {
        info!("[appearance] Setting font: {:?}", font);
        self.config_provider
            .update(Box::new(move |cfg| cfg.appearance.font = font));
        Ok(())
    }
}
