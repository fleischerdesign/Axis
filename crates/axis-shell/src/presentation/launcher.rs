use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use axis_application::use_cases::launcher::search::SearchLauncherUseCase;
use axis_domain::models::launcher::{LauncherAction, LauncherItem, LauncherStatus};
use std::rc::Rc;
use std::cell::RefCell;
use gtk4::glib;
use std::os::unix::process::CommandExt;

pub trait LauncherView {
    fn render_results(&self, results: &[LauncherItem], selected_index: Option<usize>);
    #[allow(dead_code)]
    fn clear_and_focus(&self);
}

pub struct LauncherPresenter {
    search_use_case: Arc<SearchLauncherUseCase>,
    status: Rc<RefCell<LauncherStatus>>,
    cancel_flag: Rc<RefCell<Arc<AtomicBool>>>,
    view: Rc<RefCell<Option<Rc<dyn LauncherView>>>>,
    on_close: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
}

impl LauncherPresenter {
    pub fn new(search_use_case: Arc<SearchLauncherUseCase>) -> Rc<Self> {
        Rc::new(Self {
            search_use_case,
            status: Rc::new(RefCell::new(LauncherStatus::default())),
            cancel_flag: Rc::new(RefCell::new(Arc::new(AtomicBool::new(false)))),
            view: Rc::new(RefCell::new(None)),
            on_close: Rc::new(RefCell::new(None)),
        })
    }

    pub fn bind(&self, view: Rc<dyn LauncherView>) {
        *self.view.borrow_mut() = Some(view);
    }

    pub fn on_close(&self, f: Box<dyn Fn() + 'static>) {
        *self.on_close.borrow_mut() = Some(f);
    }

    pub fn search(&self, query: &str) {
        self.cancel_flag.borrow().store(true, Ordering::SeqCst);
        let flag = Arc::new(AtomicBool::new(false));
        *self.cancel_flag.borrow_mut() = flag.clone();

        {
            let mut status = self.status.borrow_mut();
            status.query = query.to_string();
            status.is_searching = true;
            if query.trim().is_empty() {
                status.results.clear();
                status.selected_index = None;
            }
        }

        let uc = self.search_use_case.clone();
        let status = self.status.clone();
        let view = self.view.clone();
        let query = query.to_string();

        glib::spawn_future_local(async move {
            let results = uc.execute(&query).await.unwrap_or_default();
            if flag.load(Ordering::SeqCst) { return; }

            let mut s = status.borrow_mut();
            s.results = results;
            s.is_searching = false;
            s.selected_index = if s.results.is_empty() { None } else { Some(0) };

            let results_clone = s.results.clone();
            let selected = s.selected_index;
            drop(s);

            if let Some(v) = view.borrow().as_ref() {
                v.render_results(&results_clone, selected);
            }
        });
    }

    pub fn select_next(&self) {
        let mut s = self.status.borrow_mut();
        if s.results.is_empty() { return; }
        let next = s.selected_index.map_or(0, |i| (i + 1).min(s.results.len() - 1));
        s.selected_index = Some(next);
        let selected = s.selected_index;
        let results = s.results.clone();
        drop(s);

        if let Some(v) = self.view.borrow().as_ref() {
            v.render_results(&results, selected);
        }
    }

    pub fn select_prev(&self) {
        let mut s = self.status.borrow_mut();
        if s.results.is_empty() { return; }
        let prev = s.selected_index.map_or(0, |i| i.saturating_sub(1));
        s.selected_index = Some(prev);
        let selected = s.selected_index;
        let results = s.results.clone();
        drop(s);

        if let Some(v) = self.view.borrow().as_ref() {
            v.render_results(&results, selected);
        }
    }

    pub fn activate(&self, maybe_idx: Option<usize>) {
        let s = self.status.borrow();
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
        drop(s);

        if let Some(f) = self.on_close.borrow().as_ref() {
            f();
        }
    }
}
