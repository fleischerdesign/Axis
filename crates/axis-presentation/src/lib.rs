pub mod presenter;
pub mod theme;
pub mod view;

use std::cell::RefCell;
use std::rc::Rc;

pub use presenter::Presenter;
pub use view::FnView;
pub use view::View;

pub type FnCell0 = Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>;
pub type FnCell<T> = Rc<RefCell<Option<Box<dyn Fn(T) + 'static>>>>;

#[cfg(feature = "gtk")]
pub use theme::gtk_service::GtkThemeService as ThemeService;
