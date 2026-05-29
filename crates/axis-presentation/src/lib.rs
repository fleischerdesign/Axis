pub mod presenter;
pub mod theme;
pub mod view;

use std::cell::RefCell;
use std::rc::Rc;

pub use presenter::Presenter;
pub use view::FnView;
pub use view::View;

/// Callback cell for a function taking no arguments.
///
/// Thread-safe via `Rc<RefCell<Option<Box<dyn Fn()>>>>`.
/// Used for signal handlers that don't carry data (e.g. escape, close).
pub type FnCell0 = Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>;

/// Callback cell for a function taking one argument of type `T`.
pub type FnCell<T> = Rc<RefCell<Option<Box<dyn Fn(T) + 'static>>>>;

#[cfg(feature = "gtk")]
pub use theme::gtk_service::GtkThemeService as ThemeService;
