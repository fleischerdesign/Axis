use log::{info, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct ConfigWatcher;

impl ConfigWatcher {
    pub fn spawn(
        config_path: PathBuf,
        on_reload: impl Fn() + Send + 'static,
        suppress_reload: Arc<AtomicBool>,
    ) {
        std::thread::spawn(move || {
            use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};

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

                        if suppress_reload.swap(false, Ordering::SeqCst) {
                            return;
                        }

                        let now = Instant::now();
                        if now.duration_since(last_reload) < Duration::from_secs(1) {
                            return;
                        }
                        last_reload = now;

                        info!("[config-watcher] Config file changed, reloading");
                        on_reload();
                    }
                    Err(e) => warn!("[config-watcher] Watch error: {e}"),
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    warn!("[config-watcher] Failed to create watcher: {e}");
                    return;
                }
            };

            if let Some(parent) = config_path.parent() {
                if let Err(e) = watcher.watch(parent, RecursiveMode::NonRecursive) {
                    warn!("[config-watcher] Failed to watch {}: {e}", parent.display());
                    return;
                }
            }

            info!("[config-watcher] Watching {}", config_path.display());

            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        });
    }
}
