pub mod provider;
pub mod providers;

use crate::services::launcher::provider::{LauncherItem, LauncherProvider};
use crate::store::Store;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
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
                        if let Some(idx) = data.selected_index {
                            if let Some(item) = data.results.get(idx) {
                                match &item.action {
                                    crate::services::launcher::provider::LauncherAction::Exec(cmd) => {
                                        let _ = std::process::Command::new("sh")
                                            .arg("-c")
                                            .arg(cmd)
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
                            d.selected_index = if query_trimmed.is_empty() { None } else { Some(0) };
                            if query_trimmed.is_empty() {
                                d.results.clear();
                            }
                        });

                        if query_trimmed.is_empty() {
                            continue;
                        }

                        // Wir kopieren die Provider-Liste in einen lokalen Vec von Arcs.
                        // Arcs zu klonen ist billig und löst das Borrow-Problem über await.
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
                        });
                    }
                }
            }
        });
    }
}
