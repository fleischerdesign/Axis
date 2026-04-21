use axis_domain::models::launcher::{LauncherAction, LauncherItem, SearchPriority};
use axis_domain::ports::launcher::{LauncherError, LauncherSearchProvider};
use crate::adapters::launcher::util::scored_match;
use async_trait::async_trait;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct FileSearchProvider;

impl FileSearchProvider {
    fn search_dirs() -> Vec<String> {
        let mut dirs = Vec::new();

        if let Ok(home) = std::env::var("HOME") {
            dirs.push(home);
        }

        for var in [
            "XDG_DOCUMENTS_DIR",
            "XDG_DOWNLOAD_DIR",
            "XDG_DESKTOP_DIR",
            "XDG_MUSIC_DIR",
            "XDG_PICTURES_DIR",
            "XDG_VIDEOS_DIR",
        ] {
            if let Ok(dir) = std::env::var(var) {
                if !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        }

        dirs
    }

    fn file_icon(path: &str) -> &'static str {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        match ext.as_deref() {
            Some("pdf") => "application-pdf-symbolic",
            Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "webp") => "image-x-generic-symbolic",
            Some("mp4" | "mkv" | "avi" | "webm") => "video-x-generic-symbolic",
            Some("mp3" | "ogg" | "flac" | "wav") => "audio-x-generic-symbolic",
            Some("zip" | "tar" | "gz" | "xz" | "7z" | "rar") => "package-x-generic-symbolic",
            Some("rs" | "py" | "js" | "ts" | "c" | "cpp" | "h") => "text-x-script-symbolic",
            Some("txt" | "md" | "log") => "text-x-generic-symbolic",
            _ => "text-x-generic-symbolic",
        }
    }

    fn do_search(query: String) -> Vec<LauncherItem> {
        let dirs = Self::search_dirs();
        if dirs.is_empty() {
            return vec![];
        }

        let output = match Command::new("locate")
            .args(["-i", "-b", "-l", "50000", "--", &query])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
            _ => return vec![],
        };

        let query_lower = query.to_lowercase();

        output
            .lines()
            .filter_map(|path| {
                let p = Path::new(path);
                if !p.is_file() || path.contains("/.") {
                    return None;
                }
                if !dirs.iter().any(|d| path.starts_with(d.as_str())) {
                    return None;
                }

                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())?;

                if !name.to_lowercase().contains(&query_lower) {
                    return None;
                }

                let score = scored_match(&name, None, &query);
                if score == 0 {
                    return None;
                }

                Some(LauncherItem {
                    id: format!("file-{path}"),
                    title: name,
                    description: Some(path.to_string()),
                    icon_name: Self::file_icon(path).into(),
                    action: LauncherAction::Exec(format!("xdg-open '{path}'")),
                    score,
                    priority: SearchPriority::Fallback,
                })
            })
            .take(20)
            .collect()
    }
}

#[async_trait]
impl LauncherSearchProvider for FileSearchProvider {
    async fn search(&self, query: &str) -> Result<Vec<LauncherItem>, LauncherError> {
        let query = query.to_string();
        if query.is_empty() {
            return Ok(vec![]);
        }

        let (tx, rx) = tokio::sync::oneshot::channel();

        std::thread::spawn(move || {
            let results = Self::do_search(query);
            let _ = tx.send(results);
        });

        match rx.await {
            Ok(results) => Ok(results),
            Err(_) => Ok(Vec::new()),
        }
    }
}
