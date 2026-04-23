use std::sync::Arc;
use axis_domain::models::agenda::AgendaStatus;
use axis_domain::models::popups::PopupType;
use axis_domain::ports::popups::PopupProvider;
use axis_presentation::{Presenter, View};
use axis_application::use_cases::cloud::sync_calendar::SyncCalendarUseCase;
use axis_application::use_cases::cloud::sync_tasks::SyncTasksUseCase;
use axis_application::use_cases::tasks::toggle_task::ToggleTaskUseCase;
use axis_application::use_cases::tasks::delete_task::DeleteTaskUseCase;
use axis_application::use_cases::tasks::create_task::CreateTaskUseCase;
use axis_domain::models::tasks::Task;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use gtk4::glib;
use futures_util::StreamExt;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub trait AgendaView: View<AgendaStatus> {
    fn on_list_changed(&self, f: Box<dyn Fn(String) + 'static>);
    fn on_task_toggled(&self, f: Box<dyn Fn(String, bool) + 'static>);
    fn on_task_deleted(&self, f: Box<dyn Fn(String) + 'static>);
    fn on_task_created(&self, f: Box<dyn Fn(String) + 'static>);
}

pub struct AgendaPresenter {
    inner: Presenter<AgendaStatus>,
    sync_calendar_uc: Arc<SyncCalendarUseCase>,
    sync_tasks_uc: Arc<SyncTasksUseCase>,
    toggle_task_uc: Arc<ToggleTaskUseCase>,
    delete_task_uc: Arc<DeleteTaskUseCase>,
    create_task_uc: Arc<CreateTaskUseCase>,
    selected_list_id: Rc<RefCell<Option<String>>>,
    status_tx: watch::Sender<AgendaStatus>,
    is_syncing_events: Rc<Cell<bool>>,
    is_syncing_tasks: Rc<Cell<bool>>,
}

impl AgendaPresenter {
    pub fn new(
        sync_calendar_uc: Arc<SyncCalendarUseCase>,
        sync_tasks_uc: Arc<SyncTasksUseCase>,
        toggle_task_uc: Arc<ToggleTaskUseCase>,
        delete_task_uc: Arc<DeleteTaskUseCase>,
        create_task_uc: Arc<CreateTaskUseCase>,
    ) -> Self {
        let (status_tx, _) = watch::channel(AgendaStatus::default());
        let status_tx_c = status_tx.clone();

        let inner = Presenter::new(move || {
            let rx = status_tx_c.subscribe();
            Box::pin(WatchStream::new(rx))
        });

        Self { 
            inner, 
            sync_calendar_uc,
            sync_tasks_uc,
            toggle_task_uc,
            delete_task_uc,
            create_task_uc,
            selected_list_id: Rc::new(RefCell::new(None)),
            status_tx,
            is_syncing_events: Rc::new(Cell::new(false)),
            is_syncing_tasks: Rc::new(Cell::new(false)),
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

        let this_toggle = self.clone();
        view.on_task_toggled(Box::new(move |id, done| {
            this_toggle.toggle_task(id, done);
        }));

        let this_delete = self.clone();
        view.on_task_deleted(Box::new(move |id| {
            this_delete.delete_task(id);
        }));

        let this_create = self.clone();
        view.on_task_created(Box::new(move |title| {
            this_create.create_task(title);
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
        if fetch_events && self.is_syncing_events.get() { return; }
        if fetch_tasks && self.is_syncing_tasks.get() { return; }

        let mut status = self.status_tx.borrow().clone();
        if fetch_events { 
            self.is_syncing_events.set(true);
            status.is_loading_events = true; 
        }
        if fetch_tasks { 
            self.is_syncing_tasks.set(true);
            status.is_loading_tasks = true; 
        }
        let _ = self.status_tx.send(status.clone());
        self.inner.update(status.clone());

        let this = self.clone();
        let list_id = self.selected_list_id.borrow().clone();

        glib::spawn_future_local(async move {
            let mut final_status = this.status_tx.borrow().clone();

            if fetch_events {
                if let Ok(events) = this.sync_calendar_uc.execute().await {
                    final_status.events = events;
                }
                final_status.is_loading_events = false;
                this.is_syncing_events.set(false);
            }

            if fetch_tasks {
                if let Ok((lists, tasks, selected)) = this.sync_tasks_uc.execute(list_id).await {
                    final_status.task_lists = lists;
                    final_status.tasks = tasks;
                    if this.selected_list_id.borrow().is_none() {
                        *this.selected_list_id.borrow_mut() = selected.clone();
                    }
                    final_status.selected_list_id = selected;
                }
                final_status.is_loading_tasks = false;
                this.is_syncing_tasks.set(false);
            }

            this.inner.update(final_status.clone());
            let _ = this.status_tx.send(final_status);
        });
    }

    pub fn set_list(&self, list_id: String) {
        if self.selected_list_id.borrow().as_deref() == Some(&list_id) {
            return;
        }
        log::debug!("[agenda] Switching to list: {}", list_id);
        *self.selected_list_id.borrow_mut() = Some(list_id);
        let this = self.clone();
        glib::spawn_future_local(async move {
            this.refresh(false, true).await;
        });
    }

    pub fn toggle_task(&self, task_id: String, done: bool) {
        let mut status = self.status_tx.borrow().clone();
        let list_id = status.selected_list_id.clone();
        
        if let Some(task) = status.tasks.iter_mut().find(|t| t.id == task_id) {
            task.done = done;
        }
        self.inner.update(status.clone());
        let _ = self.status_tx.send(status);

        if let Some(list_id) = list_id {
            let this = self.clone();
            let uc = self.toggle_task_uc.clone();
            glib::spawn_future_local(async move {
                if let Err(e) = uc.execute(&list_id, &task_id, done).await {
                    log::error!("[agenda] Failed to toggle task: {e}");
                    this.refresh(false, true).await;
                }
            });
        }
    }

    pub fn delete_task(&self, task_id: String) {
        let mut status = self.status_tx.borrow().clone();
        let list_id = status.selected_list_id.clone();
        
        status.tasks.retain(|t| t.id != task_id);
        self.inner.update(status.clone());
        let _ = self.status_tx.send(status);

        if let Some(list_id) = list_id {
            let this = self.clone();
            let uc = self.delete_task_uc.clone();
            glib::spawn_future_local(async move {
                if let Err(e) = uc.execute(&list_id, &task_id).await {
                    log::error!("[agenda] Failed to delete task: {e}");
                    this.refresh(false, true).await;
                }
            });
        }
    }

    pub fn create_task(&self, title: String) {
        let mut status = self.status_tx.borrow().clone();
        let list_id = status.selected_list_id.clone();
        
        if let Some(ref list_id) = list_id {
            // 1. Optimistic Update
            let temp_task = Task {
                id: format!("temp-{}", uuid::Uuid::new_v4()),
                title: title.clone(),
                done: false,
                list_id: list_id.clone(),
            };
            status.tasks.insert(0, temp_task);
            self.inner.update(status.clone());
            let _ = self.status_tx.send(status);

            // 2. Sync to Google
            let this = self.clone();
            let uc = self.create_task_uc.clone();
            let list_id_c = list_id.clone();
            glib::spawn_future_local(async move {
                if let Err(e) = uc.execute(&list_id_c, &title).await {
                    log::error!("[agenda] Failed to create task: {e}");
                }
                this.refresh(false, true).await;
            });
        }
    }
}

impl Clone for AgendaPresenter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            sync_calendar_uc: self.sync_calendar_uc.clone(),
            sync_tasks_uc: self.sync_tasks_uc.clone(),
            toggle_task_uc: self.toggle_task_uc.clone(),
            delete_task_uc: self.delete_task_uc.clone(),
            create_task_uc: self.create_task_uc.clone(),
            selected_list_id: self.selected_list_id.clone(),
            status_tx: self.status_tx.clone(),
            is_syncing_events: self.is_syncing_events.clone(),
            is_syncing_tasks: self.is_syncing_tasks.clone(),
        }
    }
}
