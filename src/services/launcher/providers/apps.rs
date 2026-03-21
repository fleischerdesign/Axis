use crate::services::launcher::provider::{LauncherAction, LauncherItem, LauncherProvider};
use std::future::Future;
use std::pin::Pin;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::SystemTime;
use std::fs;

#[derive(Debug, Clone)]
struct AppEntry {
    name: String,
    exec: String,
    icon: String,
    comment: Option<String>,
}

#[derive(Debug)]
struct Cache {
    apps: Vec<AppEntry>,
    dir_mtimes: Vec<(PathBuf, SystemTime)>,
}

#[derive(Debug)]
pub struct AppProvider {
    cache: RwLock<Option<Cache>>,
}

impl Default for AppProvider {
    fn default() -> Self {
        Self { cache: RwLock::new(None) }
    }
}

impl AppProvider {
    fn get_cached_or_scan(&self) -> Vec<AppEntry> {
        {
            let guard = self.cache.read().unwrap();
            if let Some(ref cache) = *guard {
                if !self.dirs_changed(&cache.dir_mtimes) {
                    return cache.apps.clone();
                }
            }
        }

        let apps = self.scan_apps();
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

        for path in self.app_dirs() {
            if !path.exists() { continue; }
            
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if entry.path().extension().map_or(false, |ext| ext == "desktop") {
                        if let Some(app) = self.parse_desktop_file(entry.path()) {
                            // Dubletten vermeiden (z.B. wenn App in User und System Ordner liegt)
                            if !apps.iter().any(|a: &AppEntry| a.name == app.name) {
                                apps.push(app);
                            }
                        }
                    }
                }
            }
        }
        apps
    }

    fn parse_desktop_file(&self, path: PathBuf) -> Option<AppEntry> {
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

impl LauncherProvider for AppProvider {
    fn id(&self) -> &str {
        "apps"
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<LauncherItem>> + Send + 'a>> {
        Box::pin(async move {
            let query_lower = query.to_lowercase();
            let mut all_apps = self.get_cached_or_scan();
            
            // Bei leerer Suche: Alle Apps alphabetisch sortiert zurückgeben
            if query_lower.is_empty() {
                all_apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                return all_apps.into_iter().map(|app| LauncherItem {
                    id: format!("app-{}", app.name),
                    title: app.name.clone(),
                    description: app.comment.clone(),
                    icon_name: app.icon.clone(),
                    action: LauncherAction::Exec(app.exec.clone()),
                    score: 1, // Niedriger Basis-Score
                }).collect();
            }

            let mut results = Vec::new();
            for app in all_apps {
                let name_lower = app.name.to_lowercase();
                
                let mut score = 0;
                if name_lower == query_lower {
                    score = 100;
                } else if name_lower.starts_with(&query_lower) {
                    score = 80;
                } else if name_lower.contains(&query_lower) {
                    score = 50;
                } else if let Some(ref c) = app.comment {
                    if c.to_lowercase().contains(&query_lower) {
                        score = 30;
                    }
                }

                if score > 0 {
                    results.push(LauncherItem {
                        id: format!("app-{}", app.name),
                        title: app.name.clone(),
                        description: app.comment.clone(),
                        icon_name: app.icon.clone(),
                        action: LauncherAction::Exec(app.exec.clone()),
                        score,
                    });
                }
            }

            // Sortierung nach Score übernehmen wir im LauncherService
            results
        })
    }
}
