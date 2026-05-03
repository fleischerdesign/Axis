mod watcher;

use axis_domain::models::config::AxisConfig;
use axis_domain::ports::config::{ConfigError, ConfigProvider, ConfigStream};
use log::{info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct FileConfigProvider {
    base: Arc<std::sync::Mutex<AxisConfig>>,
    cli_override: AxisConfig,
    status_tx: watch::Sender<AxisConfig>,
    suppress_reload: Arc<AtomicBool>,
}

impl FileConfigProvider {
    pub fn new(cli_override: AxisConfig) -> Arc<Self> {
        let loaded = Self::load();
        let merged = Self::merge(&cli_override, &loaded);
        let (status_tx, _) = watch::channel(merged.clone());

        info!(
            "[config] Initialized — wallpaper: {:?}, accent: {:?}, scheme: {:?}, font: {:?}",
            merged.appearance.wallpaper,
            merged.appearance.accent_color,
            merged.appearance.color_scheme,
            merged.appearance.font
        );

        let base = Arc::new(std::sync::Mutex::new(loaded));
        let suppress_reload = Arc::new(AtomicBool::new(false));

        let suppress_for_watcher = suppress_reload.clone();
        let status_tx_for_watcher = status_tx.clone();
        let base_for_watcher = base.clone();
        let cli_for_watcher = cli_override.clone();

        watcher::ConfigWatcher::spawn(
            Self::config_path(),
            move || {
                let reloaded = Self::load();
                let merged = Self::merge(&cli_for_watcher, &reloaded);

                let mut guard = match base_for_watcher.lock() {
                    Ok(g) => g,
                    Err(e) => {
                        log::error!("[config] lock poisoned: {e}");
                        return;
                    }
                };
                if *guard == reloaded {
                    return;
                }
                *guard = reloaded;
                drop(guard);

                status_tx_for_watcher.send_modify(|s| *s = merged);
                info!("[config] Reloaded from external file change");
            },
            suppress_for_watcher,
        );

        Arc::new(Self {
            base,
            cli_override,
            status_tx,
            suppress_reload,
        })
    }

    fn merge(cli: &AxisConfig, file: &AxisConfig) -> AxisConfig {
        AxisConfig {
            appearance: Self::merge_appearance(&cli.appearance, &file.appearance),
            dnd: Self::merge_dnd(&cli.dnd, &file.dnd),
            nightlight: Self::merge_nightlight(&cli.nightlight, &file.nightlight),
            airplane: Self::merge_airplane(&cli.airplane, &file.airplane),
            bar: Self::merge_bar(&cli.bar, &file.bar),
            shortcuts: Self::merge_shortcuts(&cli.shortcuts, &file.shortcuts),
            continuity: Self::merge_continuity(&cli.continuity, &file.continuity),
            idle: Self::merge_idle(&cli.idle, &file.idle),
            idle_inhibit: Self::merge_idle_inhibit(&cli.idle_inhibit, &file.idle_inhibit),
        }
    }

    fn merge_appearance(
        cli: &axis_domain::models::config::AppearanceConfig,
        file: &axis_domain::models::config::AppearanceConfig,
    ) -> axis_domain::models::config::AppearanceConfig {
        axis_domain::models::config::AppearanceConfig {
            wallpaper: cli.wallpaper.clone().or(file.wallpaper.clone()),
            accent_color: if !cli.is_default_accent() {
                cli.accent_color.clone()
            } else {
                file.accent_color.clone()
            },
            color_scheme: if !cli.is_default_scheme() {
                cli.color_scheme.clone()
            } else {
                file.color_scheme.clone()
            },
            font: cli.font.clone().or(file.font.clone()),
        }
    }

    fn merge_dnd(
        cli: &axis_domain::models::config::DndConfig,
        file: &axis_domain::models::config::DndConfig,
    ) -> axis_domain::models::config::DndConfig {
        axis_domain::models::config::DndConfig {
            enabled: if cli.enabled { true } else { file.enabled },
        }
    }

    fn merge_nightlight(
        cli: &axis_domain::models::config::NightlightConfig,
        file: &axis_domain::models::config::NightlightConfig,
    ) -> axis_domain::models::config::NightlightConfig {
        let default = axis_domain::models::config::NightlightConfig::default();
        axis_domain::models::config::NightlightConfig {
            enabled: if cli.enabled != default.enabled { cli.enabled } else { file.enabled },
            temp_day: if cli.temp_day != default.temp_day { cli.temp_day } else { file.temp_day },
            temp_night: if cli.temp_night != default.temp_night { cli.temp_night } else { file.temp_night },
            sunrise: if cli.sunrise != default.sunrise { cli.sunrise.clone() } else { file.sunrise.clone() },
            sunset: if cli.sunset != default.sunset { cli.sunset.clone() } else { file.sunset.clone() },
            auto_schedule: if cli.auto_schedule != default.auto_schedule { cli.auto_schedule } else { file.auto_schedule },
            latitude: if cli.latitude != default.latitude { cli.latitude.clone() } else { file.latitude.clone() },
            longitude: if cli.longitude != default.longitude { cli.longitude.clone() } else { file.longitude.clone() },
        }
    }

    fn merge_airplane(
        cli: &axis_domain::models::config::AirplaneConfig,
        file: &axis_domain::models::config::AirplaneConfig,
    ) -> axis_domain::models::config::AirplaneConfig {
        axis_domain::models::config::AirplaneConfig {
            enabled: if cli.enabled { true } else { file.enabled },
        }
    }

    fn merge_bar(
        cli: &axis_domain::models::config::BarConfig,
        file: &axis_domain::models::config::BarConfig,
    ) -> axis_domain::models::config::BarConfig {
        let default = axis_domain::models::config::BarConfig::default();
        axis_domain::models::config::BarConfig {
            position: if cli.position != default.position { cli.position.clone() } else { file.position.clone() },
            autohide: if cli.autohide != default.autohide { cli.autohide } else { file.autohide },
            islands: Self::merge_islands(&cli.islands, &file.islands),
            show_labels: if cli.show_labels { true } else { file.show_labels },
        }
    }

    fn merge_islands(
        cli: &axis_domain::models::config::IslandVisibility,
        file: &axis_domain::models::config::IslandVisibility,
    ) -> axis_domain::models::config::IslandVisibility {
        let default = axis_domain::models::config::IslandVisibility::default();
        axis_domain::models::config::IslandVisibility {
            launcher: if cli.launcher != default.launcher { cli.launcher } else { file.launcher },
            clock: if cli.clock != default.clock { cli.clock } else { file.clock },
            status: if cli.status != default.status { cli.status } else { file.status },
            workspace: if cli.workspace != default.workspace { cli.workspace } else { file.workspace },
        }
    }

    fn merge_shortcuts(
        cli: &axis_domain::models::config::ShortcutsConfig,
        file: &axis_domain::models::config::ShortcutsConfig,
    ) -> axis_domain::models::config::ShortcutsConfig {
        let default = axis_domain::models::config::ShortcutsConfig::default();
        axis_domain::models::config::ShortcutsConfig {
            launcher: if cli.launcher != default.launcher { cli.launcher.clone() } else { file.launcher.clone() },
            quick_settings: if cli.quick_settings != default.quick_settings { cli.quick_settings.clone() } else { file.quick_settings.clone() },
            workspaces: if cli.workspaces != default.workspaces { cli.workspaces.clone() } else { file.workspaces.clone() },
            lock: if cli.lock != default.lock { cli.lock.clone() } else { file.lock.clone() },
        }
    }

    fn merge_continuity(
        cli: &axis_domain::models::config::ContinuityConfig,
        file: &axis_domain::models::config::ContinuityConfig,
    ) -> axis_domain::models::config::ContinuityConfig {
        axis_domain::models::config::ContinuityConfig {
            enabled: if cli.enabled { true } else { file.enabled },
        }
    }

    fn merge_idle(
        cli: &axis_domain::models::config::IdleConfig,
        file: &axis_domain::models::config::IdleConfig,
    ) -> axis_domain::models::config::IdleConfig {
        axis_domain::models::config::IdleConfig {
            lock_timeout_seconds: cli.lock_timeout_seconds.or(file.lock_timeout_seconds),
            blank_timeout_seconds: cli.blank_timeout_seconds.or(file.blank_timeout_seconds),
            sleep_timeout_seconds: cli.sleep_timeout_seconds.or(file.sleep_timeout_seconds),
        }
    }

    fn merge_idle_inhibit(
        cli: &axis_domain::models::config::IdleInhibitConfig,
        file: &axis_domain::models::config::IdleInhibitConfig,
    ) -> axis_domain::models::config::IdleInhibitConfig {
        axis_domain::models::config::IdleInhibitConfig {
            enabled: if cli.enabled { true } else { file.enabled },
        }
    }

    fn resolved(&self) -> AxisConfig {
        let base = match self.base.lock() {
            Ok(b) => b.clone(),
            Err(e) => {
                log::error!("[config] lock poisoned: {e}");
                return self.cli_override.clone();
            }
        };
        Self::merge(&self.cli_override, &base)
    }

    pub fn config_dir() -> PathBuf {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                PathBuf::from(home).join(".config")
            })
            .join("axis")
    }

    fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    fn load() -> AxisConfig {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<AxisConfig>(&contents) {
                Ok(config) => {
                    info!("[config] Loaded from {}", path.display());
                    config
                }
                Err(e) => {
                    warn!("[config] Failed to parse {}: {e}", path.display());
                    AxisConfig::default()
                }
            },
            Err(_) => AxisConfig::default(),
        }
    }

    fn save(config: &AxisConfig) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!("[config] Failed to create config dir: {e}");
                return;
            }
        }

        let json = match serde_json::to_string_pretty(config) {
            Ok(j) => j,
            Err(e) => {
                warn!("[config] Failed to serialize: {e}");
                return;
            }
        };

        let tmp_path = path.with_extension("tmp");
        if let Err(e) = std::fs::write(&tmp_path, &json) {
            warn!("[config] Failed to write tmp: {e}");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp_path, &path) {
            warn!("[config] Failed to rename tmp: {e}");
        }
    }
}

impl ConfigProvider for FileConfigProvider {
    fn get(&self) -> Result<AxisConfig, ConfigError> {
        Ok(self.resolved())
    }

    fn subscribe(&self) -> Result<ConfigStream, ConfigError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    fn update(&self, apply: Box<dyn FnOnce(&mut AxisConfig) + Send + 'static>) -> Result<(), ConfigError> {
        let mut guard = self.base.lock().map_err(|e| {
            ConfigError::ProviderError(format!("Lock poisoned: {e}"))
        })?;
        apply(&mut *guard);
        FileConfigProvider::save(&guard);
        self.suppress_reload.store(true, Ordering::SeqCst);
        let resolved = Self::merge(&self.cli_override, &guard);
        drop(guard);
        self.status_tx.send_modify(|s| *s = resolved);
        Ok(())
    }
}
