use crate::view::View;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::ports::StatusProvider;
use futures_util::{Stream, StreamExt};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

/// Reactive presenter that bridges an async status stream to one or more synchronous views.
///
/// Subscribes to a stream of status snapshots and renders all registered views
/// on each change. Duplicate statuses (via `PartialEq`) are filtered to avoid
/// redundant renders. Supports one-to-many binding (one presenter, many views).
///
/// Must be used from the GTK main thread because it holds `Rc`-based state.
pub struct Presenter<S>
where
    S: Clone + PartialEq + Send + 'static,
{
    views: Rc<RefCell<Vec<Box<dyn View<S>>>>>,
    current_status: Rc<RefCell<Option<S>>>,
    subscribe: Rc<dyn Fn() -> Pin<Box<dyn Stream<Item = S> + Send>> + Send + Sync>,
}

impl<S> Clone for Presenter<S>
where
    S: Clone + PartialEq + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            views: self.views.clone(),
            current_status: self.current_status.clone(),
            subscribe: self.subscribe.clone(),
        }
    }
}

impl<S> Presenter<S>
where
    S: Clone + PartialEq + Send + 'static,
{
    pub fn new(
        subscribe: impl Fn() -> Pin<Box<dyn Stream<Item = S> + Send>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            views: Rc::new(RefCell::new(vec![])),
            current_status: Rc::new(RefCell::new(None)),
            subscribe: Rc::new(subscribe),
        }
    }

    pub fn from_subscribe<F, Fut, St, E>(factory: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<St, E>> + Send + 'static,
        St: Stream<Item = S> + Send + 'static,
        E: Send + 'static,
    {
        Self::new(move || {
            let fut = factory();
            Box::pin(async_stream::stream! {
                if let Ok(stream) = fut.await {
                    let mut stream = Box::pin(stream);
                    while let Some(item) = stream.next().await {
                        yield item;
                    }
                }
            })
        })
    }

    pub fn from_subscribe_use_case<P>(use_case: Arc<SubscribeUseCase<P, S>>) -> Self
    where
        P: StatusProvider<S> + ?Sized + Send + Sync + 'static,
        P::Error: Send + 'static,
        S: Sync,
    {
        Self::from_subscribe(move || {
            let uc = use_case.clone();
            async move { uc.execute().await }
        })
    }

    /// Seeds the initial status before the first stream event arrives.
    ///
    /// When a view is added (via `add_view`), it renders immediately with
    /// this status rather than waiting for the stream.
    pub fn with_initial_status(self, status: S) -> Self {
        *self.current_status.borrow_mut() = Some(status);
        self
    }

    /// Registers a view and renders it immediately with the current status (if any).
    ///
    /// If no status is available yet (stream hasn't emitted), the view is
    /// registered but not rendered until the first status arrives.
    pub fn add_view(&self, view: Box<dyn View<S>>) {
        if let Some(status) = self.current_status.borrow().as_ref() {
            view.render(status);
        }
        self.views.borrow_mut().push(view);
    }

    pub fn current(&self) -> Option<S> {
        self.current_status.borrow().clone()
    }

    /// Imperatively sets the status and renders all views immediately.
    ///
    /// Use this for optimistic UI updates (e.g. slider drags) that should
    /// be reflected before the next stream event confirms the change.
    pub fn update(&self, status: S) {
        *self.current_status.borrow_mut() = Some(status.clone());
        self.render_all();
    }

    pub async fn bind(&self, view: Box<dyn View<S>>) {
        self.add_view(view);
        self.run_sync().await;
    }

    /// Blocks indefinitely, consuming the stream and rendering on each new status.
    ///
    /// Skips render when the new status equals the current one (via `PartialEq`).
    /// Run this in `glib::spawn_future_local` on the GTK main thread.
    pub async fn run_sync(&self) {
        let mut stream = (self.subscribe)();
        while let Some(status) = stream.next().await {
            let prev = self.current_status.borrow().clone();
            if prev.as_ref() == Some(&status) {
                continue;
            }
            *self.current_status.borrow_mut() = Some(status);
            self.render_all();
        }
    }

    /// Like `run_sync` but with a custom predicate controlling when to render.
    ///
    /// The filter receives the new status and the previous status. When it returns
    /// `false`, the internal status updates but no render occurs. Use this for
    /// feedback suppression (e.g. debouncing brightness changes from hardware).
    pub async fn run_with_filter(&self, filter: impl Fn(&S, &Option<S>) -> bool) {
        let mut stream = (self.subscribe)();
        while let Some(status) = stream.next().await {
            let prev = self.current_status.borrow().clone();
            let should_render = filter(&status, &prev);
            *self.current_status.borrow_mut() = Some(status);
            if should_render {
                self.render_all();
            }
        }
    }

    fn render_all(&self) {
        let status = self.current_status.borrow();
        if let Some(s) = status.as_ref() {
            let views = self.views.borrow();
            for view in views.iter() {
                view.render(s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::View;
    use futures_util::stream;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Spy {
        calls: Rc<RefCell<Vec<String>>>,
    }

    impl Spy {
        fn new() -> (Self, Rc<RefCell<Vec<String>>>) {
            let calls = Rc::new(RefCell::new(Vec::new()));
            (
                Self {
                    calls: calls.clone(),
                },
                calls,
            )
        }
    }

    impl View<String> for Spy {
        fn render(&self, status: &String) {
            self.calls.borrow_mut().push(status.clone());
        }
    }

    fn pending_stream() -> Presenter<String> {
        Presenter::new(|| Box::pin(stream::pending::<String>()))
    }

    #[test]
    fn add_view_renders_immediately_when_status_exists() {
        let presenter = pending_stream().with_initial_status("init".into());
        let (view, calls) = Spy::new();
        presenter.add_view(Box::new(view));
        assert_eq!(*calls.borrow(), vec!["init".to_string()]);
    }

    #[test]
    fn add_view_does_not_render_without_status() {
        let presenter = pending_stream();
        let (view, calls) = Spy::new();
        presenter.add_view(Box::new(view));
        assert!(calls.borrow().is_empty());
    }

    #[test]
    fn update_renders_to_all_views() {
        let presenter = pending_stream();
        let (v1, c1) = Spy::new();
        let (v2, c2) = Spy::new();
        presenter.add_view(Box::new(v1));
        presenter.add_view(Box::new(v2));
        presenter.update("updated".into());
        assert_eq!(*c1.borrow(), vec!["updated".to_string()]);
        assert_eq!(*c2.borrow(), vec!["updated".to_string()]);
    }

    #[test]
    fn current_returns_none_initial() {
        let presenter = pending_stream();
        assert_eq!(presenter.current(), None);
    }

    #[test]
    fn current_returns_status_after_update() {
        let presenter = pending_stream();
        presenter.update("hello".into());
        assert_eq!(presenter.current(), Some("hello".into()));
    }

    #[test]
    fn with_initial_status_seeds_current() {
        let presenter = pending_stream().with_initial_status("seed".into());
        assert_eq!(presenter.current(), Some("seed".into()));
    }

    #[tokio::test]
    async fn run_sync_filters_duplicate_statuses() {
        let items = vec!["a".to_string(), "a".to_string(), "b".to_string()];
        let presenter = Presenter::new(move || Box::pin(stream::iter(items.clone())));
        let (view, calls) = Spy::new();
        presenter.add_view(Box::new(view));
        presenter.run_sync().await;
        assert_eq!(*calls.borrow(), vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn run_with_filter_stores_all_but_renders_selectively() {
        let items = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let presenter = Presenter::new(move || Box::pin(stream::iter(items.clone())));
        let (view, calls) = Spy::new();
        presenter.add_view(Box::new(view));
        presenter
            .run_with_filter(|status, _prev| status != "y")
            .await;
        assert_eq!(*calls.borrow(), vec!["x".to_string(), "z".to_string()]);
        assert_eq!(presenter.current(), Some("z".into()));
    }
}
