use crate::widgets::callback::FnCell0;
use axis_application::use_cases::launcher::execute::ExecuteLauncherActionUseCase;
use axis_application::use_cases::launcher::search::SearchLauncherUseCase;
use axis_domain::models::launcher::LauncherStatus;
use axis_presentation::{Presenter, View};
use gtk4::glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct LauncherPresenter {
    inner: Presenter<LauncherStatus>,
    search_use_case: Arc<SearchLauncherUseCase>,
    executor: Arc<ExecuteLauncherActionUseCase>,
    cancel_flag: Rc<RefCell<Arc<AtomicBool>>>,
    on_close: FnCell0,
}

impl LauncherPresenter {
    pub fn new(
        search_use_case: Arc<SearchLauncherUseCase>,
        executor: Arc<ExecuteLauncherActionUseCase>,
    ) -> Self {
        let inner = Presenter::new(|| {
            // Launcher doesn't have an external status stream, it's driven by the presenter
            Box::pin(futures_util::stream::pending())
        })
        .with_initial_status(LauncherStatus::default());

        Self {
            inner,
            search_use_case,
            executor,
            cancel_flag: Rc::new(RefCell::new(Arc::new(AtomicBool::new(false)))),
            on_close: Rc::new(RefCell::new(None)),
        }
    }

    pub fn add_view(&self, view: Box<dyn View<LauncherStatus>>) {
        self.inner.add_view(view);
    }

    pub fn on_close(&self, f: Box<dyn Fn() + 'static>) {
        *self.on_close.borrow_mut() = Some(f);
    }

    pub fn search(&self, query: &str) {
        self.cancel_flag.borrow().store(true, Ordering::SeqCst);
        let flag = Arc::new(AtomicBool::new(false));
        *self.cancel_flag.borrow_mut() = flag.clone();

        let mut status = self.inner.current().unwrap_or_default();
        status.query = query.to_string();
        status.is_searching = true;
        if query.trim().is_empty() {
            status.results.clear();
            status.selected_index = None;
        }
        self.inner.update(status.clone());

        let uc = self.search_use_case.clone();
        let presenter = self.inner.clone();
        let query = query.to_string();

        // glib::spawn_future_local is required here because the async closure captures
        // Rc-based state (Presenter<S>) which is !Send. This is GTK's single-threaded
        // UI model and is architecturally intentional.
        glib::spawn_future_local(async move {
            let results = match uc.execute(&query).await {
                Ok(r) => r,
                Err(e) => {
                    log::error!("[launcher] search failed: {e}");
                    Default::default()
                }
            };
            if flag.load(Ordering::SeqCst) {
                return;
            }

            let mut s = presenter.current().unwrap_or_default();
            s.results = results;
            s.is_searching = false;
            s.selected_index = if s.results.is_empty() { None } else { Some(0) };
            presenter.update(s);
        });
    }

    pub fn select_next(&self) {
        let mut s = self.inner.current().unwrap_or_default();
        if s.results.is_empty() {
            return;
        }
        let next = s
            .selected_index
            .map_or(0, |i| (i + 1).min(s.results.len() - 1));
        s.selected_index = Some(next);
        self.inner.update(s);
    }

    pub fn select_prev(&self) {
        let mut s = self.inner.current().unwrap_or_default();
        if s.results.is_empty() {
            return;
        }
        let prev = s.selected_index.map_or(0, |i| i.saturating_sub(1));
        s.selected_index = Some(prev);
        self.inner.update(s);
    }

    pub fn activate(&self, maybe_idx: Option<usize>) {
        let s = self.inner.current().unwrap_or_default();
        let idx =
            maybe_idx
                .or(s.selected_index)
                .or(if !s.results.is_empty() { Some(0) } else { None });

        if let Some(idx) = idx
            && let Some(item) = s.results.get(idx)
            && let Err(e) = self.executor.execute(&item.action)
        {
            log::error!("[launcher] Failed to execute action: {e}");
        }

        if let Some(f) = self.on_close.borrow().as_ref() {
            f();
        }
    }
}
