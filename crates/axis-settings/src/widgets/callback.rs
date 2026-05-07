use std::cell::RefCell;
use std::rc::Rc;

pub type FnCell0 = Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>;
pub type FnCell<T> = Rc<RefCell<Option<Box<dyn Fn(T) + 'static>>>>;
