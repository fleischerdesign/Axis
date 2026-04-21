mod bar;
mod appearance;
mod nightlight;
mod services;
mod shortcuts;
mod continuity;
mod peer_detail;
mod about;
mod network;
mod bluetooth;

pub use bar::BarPage;
pub use appearance::AppearancePage;
pub use nightlight::NightlightPage;
pub use services::ServicesPage;
pub use shortcuts::ShortcutsPage;
pub use continuity::ContinuityPage;
pub use peer_detail::PeerDetailPage;
pub use about::AboutPage;
pub use network::NetworkPage;
pub use bluetooth::BluetoothPage;

pub use crate::page::SettingsPage;

use std::rc::Rc;
use crate::continuity_proxy::ContinuityProxy;
use crate::network_proxy::NetworkProxy;
use crate::bluetooth_proxy::BluetoothProxy;

pub fn all_pages(
    continuity: Option<&Rc<ContinuityProxy>>,
    network: Option<&Rc<NetworkProxy>>,
    bluetooth: Option<&Rc<BluetoothProxy>>,
) -> Vec<Box<dyn SettingsPage>> {
    vec![
        Box::new(BarPage),
        Box::new(AppearancePage),
        Box::new(NightlightPage),
        Box::new(ServicesPage),
        Box::new(NetworkPage::new(network)),
        Box::new(BluetoothPage::new(bluetooth)),
        Box::new(ContinuityPage::new(continuity)),
        Box::new(ShortcutsPage),
        Box::new(AboutPage),
    ]
}

/// Build all pages except the given one (used when that page needs special wiring).
pub fn all_pages_except(
    continuity: Option<&Rc<ContinuityProxy>>,
    network: Option<&Rc<NetworkProxy>>,
    bluetooth: Option<&Rc<BluetoothProxy>>,
    except_id: &str,
) -> Vec<Box<dyn SettingsPage>> {
    all_pages(continuity, network, bluetooth)
        .into_iter()
        .filter(|p| p.id() != except_id)
        .collect()
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
