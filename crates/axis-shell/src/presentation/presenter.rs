use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use futures_util::{Stream, StreamExt};

pub trait View<S> {
    fn render(&self, status: &S);
}

impl<S, T: View<S> + ?Sized> View<S> for Rc<T> {
    fn render(&self, status: &S) {
        (**self).render(status);
    }
}

impl<S, T: View<S> + ?Sized> View<S> for std::sync::Arc<T> {
    fn render(&self, status: &S) {
        (**self).render(status);
    }
}

pub struct Presenter<V: ?Sized, S>
where
    S: Clone + PartialEq + Send + 'static,
    V: View<S>,
{
    views: Rc<RefCell<Vec<Box<V>>>>,
    current_status: Rc<RefCell<Option<S>>>,
    subscribe: Arc<dyn Fn() -> Pin<Box<dyn Stream<Item = S> + Send>> + Send + Sync>,
}

impl<V: ?Sized, S> Clone for Presenter<V, S>
where
    S: Clone + PartialEq + Send + 'static,
    V: View<S>,
{
    fn clone(&self) -> Self {
        Self {
            views: self.views.clone(),
            current_status: self.current_status.clone(),
            subscribe: self.subscribe.clone(),
        }
    }
}

impl<V: ?Sized, S> Presenter<V, S>
where
    S: Clone + PartialEq + Send + 'static,
    V: View<S>,
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

    pub fn with_initial_status(self, status: S) -> Self {
        *self.current_status.borrow_mut() = Some(status);
        self
    }

    pub fn add_view(&self, view: Box<V>) {
        if let Some(status) = self.current_status.borrow().as_ref() {
            view.render(status);
        }
        self.views.borrow_mut().push(view);
    }

    pub fn current(&self) -> Option<S> {
        self.current_status.borrow().clone()
    }

    pub fn update(&self, status: S) {
        *self.current_status.borrow_mut() = Some(status);
        self.render_all();
    }

    pub async fn run(&self) {
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

    pub async fn bind(&self, view: Box<V>) {
        self.add_view(view);
        self.run().await;
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
