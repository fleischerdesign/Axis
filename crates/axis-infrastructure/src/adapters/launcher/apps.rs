use axis_domain::models::launcher::{LauncherAction, LauncherItem, SearchPriority};
use axis_domain::ports::launcher::{LauncherError, LauncherSearchProvider};
use crate::adapters::launcher::util::scored_match;
use async_trait::async_trait;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use std::fs;

#[derive(Debug, Clone)]
struct AppEntry {
    name: String,
    exec: String,
    icon: String,
    comment: Option<String>,
}

#[derive(Debug, Clone)]
struct Cache {
    apps: Vec<AppEntry>,
    dir_mtimes: Vec<(PathBuf, SystemTime)>,
}

pub struct AppSearchProvider {
    cache: Arc<RwLock<Option<Cache>>>,
}

impl AppSearchProvider {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
        }
    }

    fn do_search(apps: &[AppEntry], query: &str) -> Vec<LauncherItem> {
        let query_lower = query.to_lowercase();

        if query_lower.is_empty() {
            return apps.iter().map(|app| LauncherItem {
                id: format!("app-{}", app.name),
                title: app.name.clone(),
                description: app.comment.clone(),
                icon_name: app.icon.clone(),
                action: LauncherAction::Exec(app.exec.clone()),
                score: 1,
                priority: SearchPriority::Primary,
            }).collect();
        }

        let mut results = Vec::new();
        for app in apps {
            let score = scored_match(&app.name, app.comment.as_deref(), &query_lower);
            if score > 0 {
                results.push(LauncherItem {
                    id: format!("app-{}", app.name),
                    title: app.name.clone(),
                    description: app.comment.clone(),
                    icon_name: app.icon.clone(),
                    action: LauncherAction::Exec(app.exec.clone()),
                    score,
                    priority: SearchPriority::Primary,
                });
            }
        }

        results
    }

    fn get_cached_or_scan(&self) -> Vec<AppEntry> {
        {
            let guard = self.cache.read().unwrap();
            if let Some(ref cache) = *guard {
                if !self.dirs_changed(&cache.dir_mtimes) {
                    return cache.apps.clone();
                }
            }
        }

        let mut apps = self.scan_apps();
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        log::info!("[launcher] Found {} apps", apps.len());
        let dir_mtimes = self.get_dir_mtimes();
        *self.cache.write().unwrap() = Some(Cache { apps: apps.clone(), dir_mtimes });
        apps
    }

    fn dirs_changed(&self, cached: &[(PathBuf, SystemTime)]) -> bool {
        for (path, cached_time) in cached {
            if let Ok(meta) = fs::metadata(path) {
                if let Ok(mtime) = meta.modified() {
                    if mtime != *cached_time {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn get_dir_mtimes(&self) -> Vec<(PathBuf, SystemTime)> {
        self.app_dirs()
            .into_iter()
            .filter_map(|p| {
                let mtime = fs::metadata(&p).ok()?.modified().ok()?;
                Some((p, mtime))
            })
            .collect()
    }

    fn app_dirs(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            paths.push(PathBuf::from(data_home).join("applications"));
        } else if let Ok(home) = std::env::var("HOME") {
            paths.push(PathBuf::from(home).join(".local/share/applications"));
        }

        if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir in data_dirs.split(':') {
                paths.push(PathBuf::from(dir).join("applications"));
            }
        } else {
            paths.push(PathBuf::from("/usr/local/share/applications"));
            paths.push(PathBuf::from("/usr/share/applications"));
        }

        paths
    }

    fn scan_apps(&self) -> Vec<AppEntry> {
        let mut apps = Vec::new();
        let mut seen_names = HashSet::new();

        for path in self.app_dirs() {
            if !path.exists() { continue; }

            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if entry.path().extension().map_or(false, |ext| ext == "desktop") {
                        if let Some(app) = Self::parse_desktop_file(entry.path()) {
                            if seen_names.insert(app.name.clone()) {
                                apps.push(app);
                            }
                        }
                    }
                }
            }
        }
        apps
    }

    fn parse_desktop_file(path: PathBuf) -> Option<AppEntry> {
        let content = fs::read_to_string(path).ok()?;
        let mut name = None;
        let mut exec = None;
        let mut icon = None;
        let mut comment = None;
        let mut no_display = false;
        let mut is_app = false;

        for line in content.lines() {
            let line = line.trim();
            if line == "[Desktop Entry]" {
                is_app = true;
            } else if line.starts_with("Name=") && name.is_none() {
                name = Some(line.replace("Name=", ""));
            } else if line.starts_with("Exec=") && exec.is_none() {
                let full_exec = line.replace("Exec=", "");
                let clean_exec = full_exec.split_whitespace()
                    .filter(|s| !s.starts_with('%'))
                    .collect::<Vec<_>>()
                    .join(" ");
                exec = Some(clean_exec);
            } else if line.starts_with("Icon=") && icon.is_none() {
                icon = Some(line.replace("Icon=", ""));
            } else if line.starts_with("Comment=") && comment.is_none() {
                comment = Some(line.replace("Comment=", ""));
            } else if line == "NoDisplay=true" || line == "Type=Service" {
                no_display = true;
            }
        }

        if !is_app || no_display { return None; }

        match (name, exec) {
            (Some(n), Some(e)) => Some(AppEntry {
                name: n,
                exec: e,
                icon: icon.unwrap_or_else(|| "application-x-executable-symbolic".to_string()),
                comment,
            }),
            _ => None,
        }
    }
}

#[async_trait]
impl LauncherSearchProvider for AppSearchProvider {
    async fn search(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError> {
        let cache = self.cache.clone();
        let query = query.to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();

        std::thread::spawn(move || {
            let provider = AppSearchProvider { cache };
            let apps = provider.get_cached_or_scan();
            let results = Self::do_search(&apps, &query);
            let _ = tx.send(results);
        });

        match rx.await {
            Ok(results) => Ok(results),
            Err(_) => Ok(Vec::new()),
        }
    }
}
