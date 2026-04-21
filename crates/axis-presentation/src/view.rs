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
