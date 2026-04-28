use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use axis_application::use_cases::launcher::search::SearchLauncherUseCase;
use axis_domain::models::launcher::{LauncherAction, LauncherStatus};
use axis_presentation::{Presenter, View};
use std::rc::Rc;
use std::cell::RefCell;
use gtk4::glib;
use std::os::unix::process::CommandExt;

pub struct LauncherPresenter {
    inner: Presenter<LauncherStatus>,
    search_use_case: Arc<SearchLauncherUseCase>,
    cancel_flag: Rc<RefCell<Arc<AtomicBool>>>,
    on_close: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
}

impl LauncherPresenter {
    pub fn new(search_use_case: Arc<SearchLauncherUseCase>) -> Self {
        let inner = Presenter::new(|| {
            // Launcher doesn't have an external status stream, it's driven by the presenter
            Box::pin(futures_util::stream::pending())
        }).with_initial_status(LauncherStatus::default());

        Self {
            inner,
            search_use_case,
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

        glib::spawn_future_local(async move {
            let results = uc.execute(&query).await.unwrap_or_default();
            if flag.load(Ordering::SeqCst) { return; }

            let mut s = presenter.current().unwrap_or_default();
            s.results = results;
            s.is_searching = false;
            s.selected_index = if s.results.is_empty() { None } else { Some(0) };
            presenter.update(s);
        });
    }

    pub fn select_next(&self) {
        let mut s = self.inner.current().unwrap_or_default();
        if s.results.is_empty() { return; }
        let next = s.selected_index.map_or(0, |i| (i + 1).min(s.results.len() - 1));
        s.selected_index = Some(next);
        self.inner.update(s);
    }

    pub fn select_prev(&self) {
        let mut s = self.inner.current().unwrap_or_default();
        if s.results.is_empty() { return; }
        let prev = s.selected_index.map_or(0, |i| i.saturating_sub(1));
        s.selected_index = Some(prev);
        self.inner.update(s);
    }

    pub fn activate(&self, maybe_idx: Option<usize>) {
        let s = self.inner.current().unwrap_or_default();
        let idx = maybe_idx
            .or(s.selected_index)
            .or_else(|| if !s.results.is_empty() { Some(0) } else { None });

        if let Some(idx) = idx {
            if let Some(item) = s.results.get(idx) {
                match &item.action {
                    LauncherAction::Exec(program) => {
                        log::info!("[launcher] Executing: {program}");
                        if let Err(e) = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(program)
                            .stdin(std::process::Stdio::null())
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .process_group(0)
                            .spawn()
                        {
                            log::error!("[launcher] Failed to execute: {program} ({e})");
                        }
                    }
                    LauncherAction::OpenUrl(url) => {
                        log::info!("[launcher] Opening URL: {url}");
                        if let Err(e) = std::process::Command::new("xdg-open")
                            .arg(url)
                            .stdin(std::process::Stdio::null())
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .process_group(0)
                            .spawn()
                        {
                            log::error!("[launcher] Failed to open URL: {url} ({e})");
                        }
                    }
                    LauncherAction::Internal(cmd) => {
                        log::info!("[launcher] Internal command: {cmd}");
                    }
                }
            }
        }

        if let Some(f) = self.on_close.borrow().as_ref() {
            f();
        }
    }
}
