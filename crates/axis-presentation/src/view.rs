use std::rc::Rc;
use std::sync::Arc;

pub trait View<S> {
    fn render(&self, status: &S);
}

impl<S, T: View<S> + ?Sized> View<S> for Rc<T> {
    fn render(&self, status: &S) {
        (**self).render(status);
    }
}

impl<S, T: View<S> + ?Sized> View<S> for Arc<T> {
    fn render(&self, status: &S) {
        (**self).render(status);
    }
}

pub struct FnView<S, F: Fn(&S)> {
    f: F,
    _marker: std::marker::PhantomData<S>,
}

impl<S, F: Fn(&S)> FnView<S, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, F: Fn(&S)> View<S> for FnView<S, F> {
    fn render(&self, status: &S) {
        (self.f)(status);
    }
}
