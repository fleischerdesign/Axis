use std::rc::Rc;
use crate::proxy::SettingsProxy;

pub trait SettingsPage {
    /// Unique ID for stack navigation
    fn id(&self) -> &'static str;

    /// Display name in sidebar
    fn title(&self) -> &'static str;

    /// Icon name for sidebar
    fn icon(&self) -> &'static str;

    /// Build the page content widget
    fn build(&self, proxy: &Rc<SettingsProxy>) -> gtk4::Widget;
}
