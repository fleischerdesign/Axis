use axis_domain::models::config::NightlightConfig;
use axis_domain::models::nightlight::NightlightStatus;
use axis_domain::ports::config::ConfigProvider;
use axis_domain::ports::nightlight::{NightlightError, NightlightProvider, NightlightStream};
use async_trait::async_trait;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::WatchStream;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

enum NightlightCmd {
    Sync(NightlightConfig),
}

pub struct ConfigNightlightProvider {
    config_provider: Arc<dyn ConfigProvider>,
    status_tx: watch::Sender<NightlightStatus>,
    cmd_tx: mpsc::Sender<NightlightCmd>,
}

impl ConfigNightlightProvider {
    pub async fn new(config_provider: Arc<dyn ConfigProvider>) -> Arc<Self> {
        let available = Self::check_available();
        let initial_config = config_provider.get().expect("config get failed").nightlight.clone();
        let initial_status = Self::config_to_status(&initial_config, available);

        let (status_tx, _) = watch::channel(initial_status.clone());
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<NightlightCmd>(32);

        if initial_config.enabled {
            let _ = cmd_tx.try_send(NightlightCmd::Sync(initial_config.clone()));
        }

        let status_tx_bg = status_tx.clone();
        std::thread::spawn(move || {
            let mut child: Option<Child> = None;
            let mut current_config = NightlightConfig::default();

            while let Some(cmd) = cmd_rx.blocking_recv() {
                match cmd {
                    NightlightCmd::Sync(config) => {
                        let want_running = config.enabled;
                        let params_changed = Self::params_differ(&current_config, &config);

                        if want_running {
                            if child.is_none() || params_changed {
                                if let Some(mut c) = child.take() {
                                    let _ = c.kill();
                                    let _ = c.wait();
                                }
                                child = Self::start_wlsunset(&config);
                            }
                        } else if let Some(mut c) = child.take() {
                            let _ = c.kill();
                            let _ = c.wait();
                        }

                        current_config = config;
                        let enabled = child.is_some();
                        let _ = status_tx_bg.send(with_enabled(Self::config_to_status(&current_config, available), enabled));

                        if let Some(ref mut c) = child {
                            match c.try_wait() {
                                Ok(Some(_)) => {
                                    log::warn!("[nightlight] wlsunset exited");
                                    child = None;
                                    let _ = status_tx_bg.send(
                                        with_enabled(Self::config_to_status(&current_config, available), false),
                                    );
                                }
                                Err(e) => {
                                    log::error!("[nightlight] Error checking wlsunset: {e}");
                                    child = None;
                                    let _ = status_tx_bg.send(
                                        with_enabled(Self::config_to_status(&current_config, available), false),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            if let Some(mut c) = child {
                let _ = c.kill();
                let _ = c.wait();
            }
        });

        let provider = Arc::new(Self {
            config_provider: config_provider.clone(),
            status_tx,
            cmd_tx,
        });

        let cmd_tx_bg = provider.cmd_tx.clone();
        let mut last_config = initial_config;
        tokio::spawn(async move {
            let mut stream = config_provider.subscribe().expect("config subscribe failed");
            while let Some(config) = futures_util::StreamExt::next(&mut stream).await {
                let nl = config.nightlight.clone();
                if nl != last_config {
                    last_config = nl.clone();
                    let _ = cmd_tx_bg.send(NightlightCmd::Sync(nl)).await;
                }
            }
        });

        provider
    }

    fn config_to_status(config: &NightlightConfig, available: bool) -> NightlightStatus {
        NightlightStatus {
            enabled: config.enabled,
            available,
            temp_day: config.temp_day,
            temp_night: config.temp_night,
            schedule_enabled: config.auto_schedule
                || (!config.sunrise.is_empty() && !config.sunset.is_empty()),
            sunrise: config.sunrise.clone(),
            sunset: config.sunset.clone(),
        }
    }

    fn params_differ(a: &NightlightConfig, b: &NightlightConfig) -> bool {
        a.temp_day != b.temp_day
            || a.temp_night != b.temp_night
            || a.sunrise != b.sunrise
            || a.sunset != b.sunset
            || a.auto_schedule != b.auto_schedule
            || a.latitude != b.latitude
            || a.longitude != b.longitude
    }

    fn check_available() -> bool {
        Command::new("which")
            .arg("wlsunset")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn start_wlsunset(config: &NightlightConfig) -> Option<Child> {
        let mut cmd = Command::new("wlsunset");
        cmd.arg("-T").arg(config.temp_day.to_string());
        cmd.arg("-t").arg(config.temp_night.to_string());

        if config.auto_schedule {
            if !config.latitude.is_empty() {
                cmd.arg("-l").arg(&config.latitude);
            }
            if !config.longitude.is_empty() {
                cmd.arg("-L").arg(&config.longitude);
            }
        } else if !config.sunrise.is_empty() && !config.sunset.is_empty() {
            cmd.arg("-S").arg(format!("{}:00", config.sunrise.replace(':', "")));
            cmd.arg("-s").arg(format!("{}:00", config.sunset.replace(':', "")));
        }

        match cmd.spawn() {
            Ok(c) => {
                log::info!(
                    "[nightlight] wlsunset started (day={}K, night={}K)",
                    config.temp_day,
                    config.temp_night
                );
                Some(c)
            }
            Err(e) => {
                log::error!("[nightlight] Failed to start wlsunset: {e}");
                None
            }
        }
    }
}

fn with_enabled(mut status: NightlightStatus, enabled: bool) -> NightlightStatus {
    status.enabled = enabled;
    status
}

#[async_trait]
impl NightlightProvider for ConfigNightlightProvider {
    async fn get_status(&self) -> Result<NightlightStatus, NightlightError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<NightlightStream, NightlightError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn set_enabled(&self, enabled: bool) -> Result<(), NightlightError> {
        self.config_provider
            .update(Box::new(move |cfg| cfg.nightlight.enabled = enabled))
            .map_err(|e| NightlightError::ProviderError(e.to_string()))
    }

    async fn set_temp_day(&self, temp: u32) -> Result<(), NightlightError> {
        self.config_provider
            .update(Box::new(move |cfg| cfg.nightlight.temp_day = temp))
            .map_err(|e| NightlightError::ProviderError(e.to_string()))
    }

    async fn set_temp_night(&self, temp: u32) -> Result<(), NightlightError> {
        self.config_provider
            .update(Box::new(move |cfg| cfg.nightlight.temp_night = temp))
            .map_err(|e| NightlightError::ProviderError(e.to_string()))
    }

    async fn set_schedule(&self, sunrise: &str, sunset: &str) -> Result<(), NightlightError> {
        let sunrise = sunrise.to_string();
        let sunset = sunset.to_string();
        self.config_provider
            .update(Box::new(move |cfg| {
                cfg.nightlight.sunrise = sunrise;
                cfg.nightlight.sunset = sunset;
            }))
            .map_err(|e| NightlightError::ProviderError(e.to_string()))
    }
}
