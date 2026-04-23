use log::{info, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct ConfigWatcher;

impl ConfigWatcher {
    pub fn spawn(
        config_path: PathBuf,
        on_reload: impl Fn() + Send + Sync + 'static,
        suppress_reload: Arc<AtomicBool>,
    ) {
        std::thread::spawn(move || {
            use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};

            if let Some(parent) = config_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let mut last_reload = Instant::now() - Duration::from_secs(10);
            let on_reload = Arc::new(on_reload);

            let mut watcher = match recommended_watcher(move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        let is_target = event.paths.iter().any(|p| p.ends_with("config.json"));
                        if !is_target { return; }

                        let dominated = matches!(
                            event.kind,
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Access(_)
                        );
                        if !dominated { return; }

                        if suppress_reload.swap(false, Ordering::SeqCst) {
                            return;
                        }

                        let now = Instant::now();
                        // Debouncing: 50ms (statt 1s), damit schnelles Klicken funktioniert
                        if now.duration_since(last_reload) < Duration::from_millis(50) {
                            return;
                        }
                        last_reload = now;

                        info!("[config-watcher] Config file changed, reloading");
                        
                        let on_reload_c = on_reload.clone();
                        std::thread::spawn(move || {
                            // Kleines Delay, damit der OS-Schreibvorgang sicher fertig ist
                            std::thread::sleep(Duration::from_millis(10));
                            on_reload_c();
                        });
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
