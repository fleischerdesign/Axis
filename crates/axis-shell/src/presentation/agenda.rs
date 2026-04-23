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
        }
    }

    pub async fn bind(&self, view: Box<dyn AgendaView>) {
        let this = self.clone();
        view.on_list_changed(Box::new(move |id| {
            this.set_list(id);
        }));
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self, popup_provider: Arc<dyn PopupProvider>) {
        self.refresh(true, true).await;

        let mut stream = popup_provider.subscribe().await.unwrap_or_else(|_| Box::pin(futures_util::stream::pending()));
        let this = self.clone();
        
        glib::spawn_future_local(async move {
            while let Some(status) = stream.next().await {
                if status.active_popup == Some(PopupType::Agenda) {
                    this.refresh(true, true).await;
                }
            }
        });

        self.inner.run_sync().await;
    }

    pub async fn refresh(&self, fetch_events: bool, fetch_tasks: bool) {
        let mut status = self.status_tx.borrow().clone();
        
        if fetch_events { status.is_loading_events = true; }
        if fetch_tasks { status.is_loading_tasks = true; }
        let _ = self.status_tx.send(status.clone());
        self.inner.update(status.clone());

        let list_id = self.selected_list_id.borrow().clone();
        
        // In a real optimized scenario, we would have separate UseCases for Events and Tasks.
        // For now, we still use the SyncAgendaUseCase but we'll implement the UI feedback.
        match self.sync_use_case.execute(list_id).await {
            Ok(new_status) => {
                let mut final_status = new_status;
                final_status.is_loading_events = false;
                final_status.is_loading_tasks = false;
                
                if self.selected_list_id.borrow().is_none() {
                    *self.selected_list_id.borrow_mut() = final_status.selected_list_id.clone();
                }
                
                self.inner.update(final_status.clone());
                let _ = self.status_tx.send(final_status);
            }
            Err(e) => {
                log::error!("[agenda] Sync failed: {e}");
                let mut error_status = self.status_tx.borrow().clone();
                error_status.is_loading_events = false;
                error_status.is_loading_tasks = false;
                self.inner.update(error_status);
            }
        }
    }

    pub fn set_list(&self, list_id: String) {
        if self.selected_list_id.borrow().as_deref() == Some(&list_id) {
            return;
        }
        log::debug!("[agenda] Switching to list: {}", list_id);
        *self.selected_list_id.borrow_mut() = Some(list_id);
        let this = self.clone();
        glib::spawn_future_local(async move {
            this.refresh(false, true).await; // Fast refresh: only tasks
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
        }
    }
}
