pub mod provider;
pub mod providers;

use crate::services::launcher::provider::{LauncherAction, LauncherItem, LauncherProvider};
use crate::services::Service;
use crate::store::ServiceStore;
use log::{error, info};
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use std::process::{Command, Stdio};
use std::os::unix::process::CommandExt;
use async_channel::Sender;
use gtk4::glib;

/// Describes what changed in the last update so the UI can avoid
/// a full list rebuild when only the selection moved.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum LauncherUpdate {
    #[default]
    Results,
    SelectionOnly,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LauncherData {
    pub query: String,
    pub results: Vec<LauncherItem>,
    pub selected_index: Option<usize>,
    pub is_searching: bool,
    pub update_kind: LauncherUpdate,
}

pub enum LauncherCmd {
    Search(String),
    SelectNext,
    SelectPrev,
    Activate(Option<usize>), // Optionaler Index für Mausklicks
}

pub struct LauncherService {
    providers: Rc<RefCell<Vec<Arc<dyn LauncherProvider>>>>,
}

impl Service for LauncherService {
    type Data = LauncherData;
    type Cmd = LauncherCmd;

    fn spawn() -> (ServiceStore<Self::Data>, Sender<Self::Cmd>) {
        let (cmd_tx, cmd_rx) = async_channel::unbounded();
        let store: ServiceStore<LauncherData> = ServiceStore::new_manual(Default::default());
        let service = Self::new();
        service.add_provider(Arc::new(providers::apps::AppProvider::default()));
        service.add_provider(Arc::new(providers::files::FileSearchProvider));
        service.add_provider(Arc::new(providers::web::WebSearchProvider::default()));

        let providers_ref = service.providers.clone();
        let data_store = store.store.clone();

        glib::spawn_future_local(async move {
            while let Ok(cmd) = cmd_rx.recv().await {
                match cmd {
                    LauncherCmd::SelectNext => {
                        data_store.update(|d| {
                            if d.results.is_empty() { return; }
                            let next = d.selected_index.map_or(0, |i| (i + 1).min(d.results.len() - 1));
                            d.selected_index = Some(next);
                            d.update_kind = LauncherUpdate::SelectionOnly;
                        });
                    }
                    LauncherCmd::SelectPrev => {
                        data_store.update(|d| {
                            if d.results.is_empty() { return; }
                            let prev = d.selected_index.map_or(0, |i| i.saturating_sub(1));
                            d.selected_index = Some(prev);
                            d.update_kind = LauncherUpdate::SelectionOnly;
                        });
                    }
                    LauncherCmd::Activate(maybe_idx) => {
                        let data = data_store.get();
                        let idx_to_activate = maybe_idx
                            .or(data.selected_index)
                            .or_else(|| if !data.results.is_empty() { Some(0) } else { None });

                        if let Some(idx) = idx_to_activate {
                            if let Some(item) = data.results.get(idx) {
                                match &item.action {
                                    LauncherAction::Exec(program) => {
                                        info!("[launcher] Executing: {program}");
                                        match Command::new("sh")
                                            .arg("-c")
                                            .arg(program)
                                            .stdin(Stdio::null())
                                            .stdout(Stdio::null())
                                            .stderr(Stdio::null())
                                            .process_group(0)
                                            .spawn()
                                        {
                                            Ok(_) => {}
                                            Err(e) => error!("[launcher] Failed to execute: {program} ({e})"),
                                        }
                                    }
                                    LauncherAction::OpenUrl(url) => {
                                        info!("[launcher] Opening URL: {url}");
                                        match Command::new("xdg-open")
                                            .arg(url)
                                            .stdin(Stdio::null())
                                            .stdout(Stdio::null())
                                            .stderr(Stdio::null())
                                            .process_group(0)
                                            .spawn()
                                        {
                                            Ok(_) => {}
                                            Err(e) => error!("[launcher] Failed to open URL: {url} ({e})"),
                                        }
                                    }
                                    LauncherAction::Internal(cmd) => {
                                        info!("[launcher] Internal command: {cmd}");
                                    }
                                }
                            }
                        }
                    }
                    LauncherCmd::Search(query) => {
                        let query_trimmed = query.trim().to_string();

                        data_store.update(|d| {
                            d.query = query.clone();
                            d.is_searching = true;
                            if query_trimmed.is_empty() {
                                d.results.clear();
                                d.selected_index = None;
                            }
                        });

                        let active_providers: Vec<Arc<dyn LauncherProvider>> = providers_ref.borrow().clone();
                        let mut all_results = Vec::new();

                        // Provider werden sequenziell gefragt, aber UI wird
                        // nach JEDEM Provider aktualisiert — schnelle Provider
                        // (Apps) zeigen sofort, langsame (Files) kommen dazu.
                        for p in active_providers {
                            let mut results = p.search(&query_trimmed).await;
                            all_results.append(&mut results);

                            // Streaming-Update: UI sofort aktualisieren
                            all_results.sort_by(|a, b| {
                                b.priority.cmp(&a.priority).then_with(|| b.score.cmp(&a.score))
                            });
                            let snapshot = all_results.clone();
                            data_store.update(move |d| {
                                d.results = snapshot;
                                d.is_searching = true; // Noch nicht fertig
                                d.selected_index = if d.results.is_empty() { None } else { Some(0) };
                                d.update_kind = LauncherUpdate::Results;
                            });
                        }

                        // Finale Markierung
                        data_store.update(|d| {
                            d.is_searching = false;
                        });
                    }
                }
            }
        });

        // Return the EXACT ServiceStore the launcher writes to
        (store, cmd_tx)
    }
}

impl LauncherService {
    pub fn new() -> Self {
        Self {
            providers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn add_provider(&self, provider: Arc<dyn LauncherProvider>) {
        self.providers.borrow_mut().push(provider);
    }
}
