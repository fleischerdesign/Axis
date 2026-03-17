pub mod provider;
pub mod providers;

use crate::services::launcher::provider::{LauncherItem, LauncherProvider};
use crate::store::Store;
use std::rc::Rc;
use std::cell::RefCell;
use async_channel::Receiver;
use gtk4::glib;

#[derive(Debug, Clone, Default)]
pub struct LauncherData {
    pub query: String,
    pub results: Vec<LauncherItem>,
    pub is_searching: bool,
}

pub enum LauncherCmd {
    Search(String),
}

pub struct LauncherService {
    providers: Rc<RefCell<Vec<Box<dyn LauncherProvider>>>>,
    store: Store<LauncherData>,
}

impl LauncherService {
    pub fn new(store: Store<LauncherData>) -> Self {
        Self {
            providers: Rc::new(RefCell::new(Vec::new())),
            store,
        }
    }

    pub fn add_provider(&self, provider: Box<dyn LauncherProvider>) {
        self.providers.borrow_mut().push(provider);
    }

    pub fn start(&self, rx: Receiver<LauncherCmd>) {
        let providers = self.providers.clone();
        let store = self.store.clone();

        // Benutze GLib's lokalen Executor statt Tokio Spawn (wegen Thread-Safety von Store/Rc)
        glib::spawn_future_local(async move {
            while let Ok(cmd) = rx.recv().await {
                match cmd {
                    LauncherCmd::Search(query) => {
                        let query_trimmed = query.trim().to_lowercase();
                        
                        store.update(|d| {
                            d.query = query.clone();
                            d.is_searching = !query_trimmed.is_empty();
                            if query_trimmed.is_empty() {
                                d.results.clear();
                            }
                        });

                        if query_trimmed.is_empty() {
                            continue;
                        }

                        let mut all_results = Vec::new();
                        // Da wir auf dem Main-Thread sind, müssen wir hier vorsichtig sein mit langen Suchen.
                        // Für lokale Apps ist es ok, für Web-Suchen später sollten wir die Provider-Search 
                        // in tokio::spawn_blocking auslagern und das Ergebnis per Channel zurückholen.
                        
                        let current_providers = providers.borrow();
                        for p in current_providers.iter() {
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
