mod bar;
mod appearance;
mod nightlight;
mod services;
mod shortcuts;
mod continuity;

pub use bar::BarPage;
pub use appearance::AppearancePage;
pub use nightlight::NightlightPage;
pub use services::ServicesPage;
pub use shortcuts::ShortcutsPage;
pub use continuity::ContinuityPage;

use std::rc::Rc;
use crate::page::SettingsPage;
use crate::continuity_proxy::ContinuityProxy;

pub fn all_pages(continuity: Option<&Rc<ContinuityProxy>>) -> Vec<Box<dyn SettingsPage>> {
    vec![
        Box::new(BarPage),
        Box::new(AppearancePage),
        Box::new(NightlightPage),
        Box::new(ServicesPage),
        Box::new(ContinuityPage::new(continuity)),
        Box::new(ShortcutsPage),
    ]
}

pub fn create_sidebar_row(title: &str, icon: &str, id: &str) -> libadwaita::ActionRow {
    use gtk4::prelude::*;
    use libadwaita::prelude::*;

    let row = libadwaita::ActionRow::builder()
        .title(title)
        .activatable(true)
        .build();
    let icon_widget = gtk4::Image::from_icon_name(icon);
    icon_widget.set_margin_start(8);
    icon_widget.set_margin_end(8);
    row.add_prefix(&icon_widget);
    row.set_widget_name(id);
    row
}
