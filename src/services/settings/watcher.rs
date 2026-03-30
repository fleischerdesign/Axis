use async_channel::Sender;
use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::SettingsCmd;
use super::config::ConfigManager;

pub struct ConfigWatcher;

impl ConfigWatcher {
    pub fn spawn(cmd_tx: Sender<SettingsCmd>, suppress_reload: Arc<AtomicBool>) {
        std::thread::spawn(move || {
            use notify::{recommended_watcher, RecursiveMode, Watcher, EventKind};

            let config_path = ConfigManager::config_path();

            if let Some(parent) = config_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let mut last_reload = Instant::now() - Duration::from_secs(10);

            let mut watcher = match recommended_watcher(move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        let dominated = matches!(
                            event.kind,
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                        );
                        if !dominated {
                            return;
                        }

                        // Suppress if SettingsService just wrote the file
                        if suppress_reload.swap(false, Ordering::SeqCst) {
                            return;
                        }

                        // Debounce: ignore events within 1s of last reload
                        let now = Instant::now();
                        if now.duration_since(last_reload) < Duration::from_secs(1) {
                            return;
                        }
                        last_reload = now;

                        info!("[config-watcher] Config file changed, reloading");
                        let _ = cmd_tx.try_send(SettingsCmd::Reload);
                    }
                    Err(e) => log::warn!("[config-watcher] Watch error: {e}"),
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    log::warn!("[config-watcher] Failed to create watcher: {e}");
                    return;
                }
            };

            if let Some(parent) = config_path.parent() {
                if let Err(e) = watcher.watch(parent, RecursiveMode::NonRecursive) {
                    log::warn!("[config-watcher] Failed to watch {}: {e}", parent.display());
                    return;
                }
            }

            info!("[config-watcher] Watching {}", config_path.display());

            // Keep thread alive
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        });
    }
}
