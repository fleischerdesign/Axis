use crate::adapters::launcher::util::scored_match;
use async_trait::async_trait;
use axis_domain::models::launcher::{LauncherAction, LauncherItem, SearchPriority};
use axis_domain::ports::launcher::{LauncherError, LauncherSearchProvider};
use log::info;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

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
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cache: Arc::new(RwLock::new(None)),
        })
    }

    fn do_search(apps: &[AppEntry], query: &str) -> Vec<LauncherItem> {
        let query_lower = query.to_lowercase();

        if query_lower.is_empty() {
            return apps
                .iter()
                .map(|app| LauncherItem {
                    id: format!("app-{}", app.name),
                    title: app.name.clone(),
                    description: app.comment.clone(),
                    icon_name: app.icon.clone(),
                    action: parse_desktop_exec(&app.exec),
                    score: 1,
                    priority: SearchPriority::Primary,
                })
                .collect();
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
                    action: parse_desktop_exec(&app.exec),
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
            if let Some(ref cache) = *guard
                && !self.dirs_changed(&cache.dir_mtimes)
            {
                return cache.apps.clone();
            }
        }

        let mut apps = self.scan_apps();
        apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        info!("[launcher] Found {} apps", apps.len());
        let dir_mtimes = self.get_dir_mtimes();
        *self.cache.write().unwrap() = Some(Cache {
            apps: apps.clone(),
            dir_mtimes,
        });
        apps
    }

    fn dirs_changed(&self, cached: &[(PathBuf, SystemTime)]) -> bool {
        for (path, cached_time) in cached {
            if let Ok(meta) = fs::metadata(path)
                && let Ok(mtime) = meta.modified()
                && mtime != *cached_time
            {
                return true;
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
            if !path.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if entry.path().extension().is_some_and(|ext| ext == "desktop")
                        && let Some(app) = Self::parse_desktop_file(entry.path())
                        && seen_names.insert(app.name.clone())
                    {
                        apps.push(app);
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
                let clean_exec = full_exec
                    .split_whitespace()
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

        if !is_app || no_display {
            return None;
        }

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

fn parse_desktop_exec(exec: &str) -> LauncherAction {
    let mut cleaned = String::new();
    let chars: Vec<char> = exec.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                '\\' => {
                    cleaned.push('\\');
                    i += 2;
                }
                '%' => {
                    cleaned.push('%');
                    i += 2;
                }
                _ => {
                    cleaned.push(chars[i]);
                    i += 1;
                }
            }
        } else if chars[i] == '%' && i + 1 < chars.len() {
            match chars[i + 1] {
                'f' | 'u' | 'F' | 'U' | 'd' | 'D' | 'i' | 'c' | 'k' => {
                    i += 2;
                }
                _ => {
                    cleaned.push(chars[i]);
                    i += 1;
                }
            }
        } else {
            cleaned.push(chars[i]);
            i += 1;
        }
    }

    let parts = shell_split(&cleaned);
    LauncherAction::Exec(parts)
}

fn shell_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;

    while i < chars.len() {
        match chars[i] {
            '\'' if !in_double => {
                in_single = !in_single;
                i += 1;
            }
            '"' if !in_single => {
                in_double = !in_double;
                i += 1;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
                i += 1;
            }
            c => {
                current.push(c);
                i += 1;
            }
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_split_simple() {
        assert_eq!(shell_split("firefox"), vec!["firefox"]);
        assert_eq!(shell_split("a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn shell_split_single_quotes() {
        assert_eq!(
            shell_split("echo 'hello world'"),
            vec!["echo", "hello world"]
        );
    }

    #[test]
    fn shell_split_double_quotes() {
        assert_eq!(
            shell_split("echo \"hello world\""),
            vec!["echo", "hello world"]
        );
    }

    #[test]
    fn shell_split_mixed_quotes() {
        assert_eq!(shell_split("echo 'a' \"b\""), vec!["echo", "a", "b"]);
    }

    #[test]
    fn shell_split_empty_string() {
        assert!(shell_split("").is_empty());
    }

    #[test]
    fn shell_split_multiple_spaces() {
        assert_eq!(shell_split("a   b"), vec!["a", "b"]);
    }

    #[test]
    fn parse_desktop_exec_simple() {
        let action = parse_desktop_exec("firefox");
        assert_eq!(action, LauncherAction::Exec(vec!["firefox".into()]));
    }

    #[test]
    fn parse_desktop_exec_strips_field_codes() {
        let action = parse_desktop_exec("firefox %u");
        assert_eq!(action, LauncherAction::Exec(vec!["firefox".into()]));

        let action = parse_desktop_exec("gvim -f %f");
        assert_eq!(
            action,
            LauncherAction::Exec(vec!["gvim".into(), "-f".into()])
        );

        let action = parse_desktop_exec("app %f %u %F %U %d %D %i %c %k");
        assert_eq!(action, LauncherAction::Exec(vec!["app".into()]));
    }

    #[test]
    fn parse_desktop_exec_preserves_escaped_percent() {
        let action = parse_desktop_exec("echo \\%s");
        assert_eq!(
            action,
            LauncherAction::Exec(vec!["echo".into(), "%s".into()])
        );
    }

    #[test]
    fn parse_desktop_exec_preserves_escaped_backslash() {
        let action = parse_desktop_exec("echo \\\\");
        assert_eq!(
            action,
            LauncherAction::Exec(vec!["echo".into(), "\\".into()])
        );
    }

    #[test]
    fn parse_desktop_exec_with_quoted_args() {
        let action = parse_desktop_exec("app --arg \"hello world\" %u");
        assert_eq!(
            action,
            LauncherAction::Exec(vec!["app".into(), "--arg".into(), "hello world".into()])
        );
    }
}
