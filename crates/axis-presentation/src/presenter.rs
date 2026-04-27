use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use futures_util::{Stream, StreamExt};
use crate::view::View;

pub struct Presenter<S>
where
    S: Clone + PartialEq + Send + 'static,
{
    views: Rc<RefCell<Vec<Box<dyn View<S>>>>>,
    current_status: Rc<RefCell<Option<S>>>,
    subscribe: Arc<dyn Fn() -> Pin<Box<dyn Stream<Item = S> + Send>> + Send + Sync>,
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
            subscribe: Arc::new(subscribe),
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

    pub fn with_initial_status(self, status: S) -> Self {
        *self.current_status.borrow_mut() = Some(status);
        self
    }

    pub fn add_view(&self, view: Box<dyn View<S>>) {
        if let Some(status) = self.current_status.borrow().as_ref() {
            view.render(status);
        }
        self.views.borrow_mut().push(view);
    }

    pub fn current(&self) -> Option<S> {
        self.current_status.borrow().clone()
    }

    pub fn update(&self, status: S) {
        *self.current_status.borrow_mut() = Some(status.clone());
        self.render_all();
    }

    pub async fn bind(&self, view: Box<dyn View<S>>) {
        self.add_view(view);
        self.run_sync().await;
    }

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
