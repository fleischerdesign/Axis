pub mod provider;
pub mod providers;

use crate::services::launcher::provider::{LauncherAction, LauncherItem, LauncherProvider};
use crate::services::Service;
use crate::store::{ServiceStore, Store};
use log::{error, info};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
    search_cancel: Rc<RefCell<Arc<AtomicBool>>>,
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
        let search_cancel = service.search_cancel.clone();
        let data_store = store.store.clone();

        // Command Queue für sofortige Verarbeitung
        let pending_cmd: Rc<RefCell<Option<LauncherCmd>>> = Rc::new(RefCell::new(None));

        // Idle-Callback für sofortige Verarbeitung
        let pending_cmd_clone = pending_cmd.clone();
        let providers_ref_clone = providers_ref.clone();
        let search_cancel_clone = search_cancel.clone();
        let data_store_clone = data_store.clone();

        glib::spawn_future_local(async move {
            while let Ok(cmd) = cmd_rx.recv().await {
                *pending_cmd_clone.borrow_mut() = Some(cmd);
                let pending = pending_cmd_clone.clone();
                let providers = providers_ref_clone.clone();
                let cancel = search_cancel_clone.clone();
                let store = data_store_clone.clone();

                glib::idle_add_local(move || {
                    if let Some(cmd) = pending.borrow_mut().take() {
                        Self::handle_cmd(cmd, &providers, &cancel, &store);
                    }
                    glib::ControlFlow::Break
                });
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
            search_cancel: Rc::new(RefCell::new(Arc::new(AtomicBool::new(false)))),
        }
    }

    pub fn add_provider(&self, provider: Arc<dyn LauncherProvider>) {
        self.providers.borrow_mut().push(provider);
    }

    fn handle_cmd(
        cmd: LauncherCmd,
        providers_ref: &Rc<RefCell<Vec<Arc<dyn LauncherProvider>>>>,
        search_cancel: &Rc<RefCell<Arc<AtomicBool>>>,
        data_store: &Store<LauncherData>,
    ) {
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
                info!("[launcher] Search started for: '{}'", query_trimmed);

                // Vorherige Suche abbrechen
                search_cancel.borrow().store(true, Ordering::SeqCst);
                let cancel = Arc::new(AtomicBool::new(false));
                *search_cancel.borrow_mut() = cancel.clone();

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

                // Alle Provider parallel starten — Ergebnisse kommen
                // in der Reihenfolge, in der sie fertig werden.
                let (result_tx, result_rx) = async_channel::unbounded();
                for p in active_providers {
                    let tx = result_tx.clone();
                    let query = query_trimmed.clone();
                    let cancel = cancel.clone();
                    glib::spawn_future_local(async move {
                        let results = p.search(&query).await;
                        if !cancel.load(Ordering::SeqCst) {
                            let _ = tx.send(results).await;
                        }
                    });
                }
                drop(result_tx); // Sender schließen, wenn alle Provider gestartet sind

                // Ergebnisse in einem separaten Future sammeln
                let data_store_clone = data_store.clone();
                glib::spawn_future_local(async move {
                    while let Ok(mut results) = result_rx.recv().await {
                        info!("[launcher] Received {} results", results.len());
                        all_results.append(&mut results);

                        // Streaming-Update: UI sofort aktualisieren
                        all_results.sort_by(|a, b| {
                            b.priority.cmp(&a.priority).then_with(|| b.score.cmp(&a.score))
                        });
                        let snapshot = all_results.clone();
                        data_store_clone.update(move |d| {
                            d.results = snapshot;
                            d.is_searching = true; // Noch nicht fertig
                            d.selected_index = if d.results.is_empty() { None } else { Some(0) };
                            d.update_kind = LauncherUpdate::Results;
                        });
                    }

                    info!("[launcher] Search completed for: '{}'", query_trimmed);

                    // Finale Markierung
                    data_store_clone.update(|d| {
                        d.is_searching = false;
                    });
                });
            }
        }
    }
}
