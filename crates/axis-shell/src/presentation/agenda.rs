use std::sync::Arc;
use axis_domain::models::agenda::AgendaStatus;
use axis_domain::models::popups::PopupType;
use axis_domain::ports::popups::PopupProvider;
use axis_presentation::{Presenter, View};
use axis_application::use_cases::cloud::sync_agenda::SyncAgendaUseCase;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use gtk4::glib;
use futures_util::StreamExt;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub trait AgendaView: View<AgendaStatus> {
    fn on_list_changed(&self, f: Box<dyn Fn(String) + 'static>);
    fn on_task_toggled(&self, f: Box<dyn Fn(String, bool) + 'static>);
}

pub struct AgendaPresenter {
    inner: Presenter<AgendaStatus>,
    sync_use_case: Arc<SyncAgendaUseCase>,
    selected_list_id: Rc<RefCell<Option<String>>>,
    status_tx: watch::Sender<AgendaStatus>,
    is_syncing: Rc<Cell<bool>>,
}

impl AgendaPresenter {
    pub fn new(sync_use_case: Arc<SyncAgendaUseCase>) -> Self {
        let (status_tx, _) = watch::channel(AgendaStatus::default());
        let status_tx_c = status_tx.clone();

        let inner = Presenter::new(move || {
            let rx = status_tx_c.subscribe();
            Box::pin(WatchStream::new(rx))
        });

        Self { 
            inner, 
            sync_use_case,
            selected_list_id: Rc::new(RefCell::new(None)),
            status_tx,
            is_syncing: Rc::new(Cell::new(false)),
        }
    }

    pub fn add_view(&self, view: Box<dyn View<AgendaStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn bind(&self, view: Box<dyn AgendaView>) {
        let this = self.clone();
        view.on_list_changed(Box::new(move |id| {
            this.set_list(id);
        }));

        self.inner.add_view(view);
    }

    pub async fn run_sync(&self, popup_provider: Arc<dyn PopupProvider>) {
        // Initial refresh
        self.refresh().await;

        let mut stream = popup_provider.subscribe().await.unwrap_or_else(|_| Box::pin(futures_util::stream::pending()));
        let this = self.clone();
        
        glib::spawn_future_local(async move {
            while let Some(status) = stream.next().await {
                if status.active_popup == Some(PopupType::Agenda) {
                    log::debug!("[agenda] Popup opened, refreshing data");
                    this.refresh().await;
                }
            }
        });

        self.inner.run_sync().await;
    }

    pub async fn refresh(&self) {
        if self.is_syncing.get() {
            log::debug!("[agenda] Sync already in progress, skipping");
            return;
        }
        self.is_syncing.set(true);

        let list_id = self.selected_list_id.borrow().clone();
        log::debug!("[agenda] Fetching fresh data from Google (list_id: {:?})", list_id);
        match self.sync_use_case.execute(list_id).await {
            Ok(status) => {
                log::info!("[agenda] Sync successful: {} events, {} tasks", status.events.len(), status.tasks.len());
                if self.selected_list_id.borrow().is_none() {
                    *self.selected_list_id.borrow_mut() = status.selected_list_id.clone();
                }
                
                self.inner.update(status.clone());
                let _ = self.status_tx.send(status);
            }
            Err(e) => log::error!("[agenda] Sync failed: {e}"),
        }
        self.is_syncing.set(false);
    }

    pub fn set_list(&self, list_id: String) {
        if self.selected_list_id.borrow().as_deref() == Some(&list_id) {
            return;
        }
        log::debug!("[agenda] List changed by user: {}", list_id);
        *self.selected_list_id.borrow_mut() = Some(list_id);
        let this = self.clone();
        glib::spawn_future_local(async move {
            this.refresh().await;
        });
    }
}

impl Clone for AgendaPresenter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            sync_use_case: self.sync_use_case.clone(),
            selected_list_id: self.selected_list_id.clone(),
            status_tx: self.status_tx.clone(),
            is_syncing: self.is_syncing.clone(),
        }
    }
}
