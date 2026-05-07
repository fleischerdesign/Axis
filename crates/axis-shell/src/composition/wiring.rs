use super::presenters::Presenters;
use super::providers::Providers;
use super::use_cases::UseCases;
use axis_infrastructure::adapters::lock::LockGtkHandle;
use std::cell::OnceCell;
use std::rc::Rc;

pub struct WiringArgs<'a> {
    pub app: &'a libadwaita::Application,
    pub p: &'a Providers,
    pub uc: &'a UseCases,
    pub pres: &'a Presenters,
    pub rt: &'a tokio::runtime::Runtime,
    pub theme_provider: Rc<OnceCell<Rc<gtk4::CssProvider>>>,
    pub lock_gtk_handle: LockGtkHandle,
    pub start_locked: bool,
}

pub fn wire(args: WiringArgs) {
    let _unused = args;
    unimplemented!("wiring::wire")
}
