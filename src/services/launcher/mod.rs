pub mod provider;
pub mod providers;

use crate::services::launcher::provider::{LauncherItem, LauncherProvider};
use crate::store::Store;
use gtk4::prelude::*;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use async_channel::Receiver;
use gtk4::glib;

#[derive(Debug, Clone, Default)]
pub struct LauncherData {
    pub query: String,
    pub results: Vec<LauncherItem>,
    pub selected_index: Option<usize>,
    pub is_searching: bool,
}

pub enum LauncherCmd {
    Search(String),
    SelectNext,
    SelectPrev,
    Activate,
}

pub struct LauncherService {
    providers: Rc<RefCell<Vec<Arc<dyn LauncherProvider>>>>,
    store: Store<LauncherData>,
}

impl LauncherService {
    pub fn new(store: Store<LauncherData>) -> Self {
        Self {
            providers: Rc::new(RefCell::new(Vec::new())),
            store,
        }
    }

    pub fn add_provider(&self, provider: Arc<dyn LauncherProvider>) {
        self.providers.borrow_mut().push(provider);
    }

    pub fn start(&self, rx: Receiver<LauncherCmd>) {
        let providers_ref = self.providers.clone();
        let store = self.store.clone();

        glib::spawn_future_local(async move {
            while let Ok(cmd) = rx.recv().await {
                match cmd {
                    LauncherCmd::SelectNext => {
                        store.update(|d| {
                            if d.results.is_empty() { return; }
                            let next = d.selected_index.map_or(0, |i| (i + 1).min(d.results.len() - 1));
                            d.selected_index = Some(next);
                        });
                    }
                    LauncherCmd::SelectPrev => {
                        store.update(|d| {
                            if d.results.is_empty() { return; }
                            let prev = d.selected_index.map_or(0, |i| i.saturating_sub(1));
                            d.selected_index = Some(prev);
                        });
                    }
                    LauncherCmd::Activate => {
                        let data = store.get();
                        
                        let idx_to_activate = data.selected_index.or_else(|| {
                            if !data.results.is_empty() { Some(0) } else { None }
                        });

                        if let Some(idx) = idx_to_activate {
                            if let Some(item) = data.results.get(idx) {
                                match &item.action {
                                    crate::services::launcher::provider::LauncherAction::Exec(cmd) => {
                                        println!("Launcher: Bulletproof Start von '{}'", cmd);
                                        
                                        // 1. Command vorbereiten
                                        // 2. I/O umleiten nach /dev/null (verhindert Hängenbleiben)
                                        // 3. process_group(0) erstellt eine neue Session (detaching)
                                        let _ = Command::new("sh")
                                            .arg("-c")
                                            .arg(cmd)
                                            .stdin(Stdio::null())
                                            .stdout(Stdio::null())
                                            .stderr(Stdio::null())
                                            .process_group(0) 
                                            .spawn();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    LauncherCmd::Search(query) => {
                        let query_trimmed = query.trim().to_lowercase();
                        
                        store.update(|d| {
                            d.query = query.clone();
                            d.is_searching = !query_trimmed.is_empty();
                            if query_trimmed.is_empty() {
                                d.results.clear();
                                d.selected_index = None;
                            }
                        });

                        if query_trimmed.is_empty() {
                            continue;
                        }

                        let active_providers: Vec<Arc<dyn LauncherProvider>> = providers_ref.borrow().clone();
                        let mut all_results = Vec::new();

                        for p in active_providers {
                            let mut results = p.search(&query_trimmed).await;
                            all_results.append(&mut results);
                        }

                        all_results.sort_by(|a, b| b.score.cmp(&a.score));

                        store.update(|d| {
                            d.results = all_results;
                            d.is_searching = false;
                            d.selected_index = if d.results.is_empty() { None } else { Some(0) };
                        });
                    }
                }
            }
        });
    }
}
