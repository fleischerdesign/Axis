use super::providers::Providers;
use super::use_cases::UseCases;
use crate::presentation::agenda::AgendaPresenter;
use crate::presentation::appearance::AppearancePresenter;
use crate::presentation::audio::AudioPresenter;
use crate::presentation::auto_hide::AutoHidePresenter;
use crate::presentation::battery::BatteryPresenter;
use crate::presentation::bluetooth::BluetoothPresenter;
use crate::presentation::brightness::BrightnessPresenter;
use crate::presentation::clock::ClockPresenter;
use crate::presentation::continuity::ContinuityPresenter;
use crate::presentation::launcher::LauncherPresenter;
use crate::presentation::lock::LockPresenter;
use crate::presentation::mpris::MprisPresenter;
use crate::presentation::network::NetworkPresenter;
use crate::presentation::nightlight::NightlightPresenter;
use crate::presentation::notifications::NotificationPresenter;
use crate::presentation::popups::PopupPresenter;
use crate::presentation::toggle::TogglePresenter;
use crate::presentation::tray::TrayPresenter;
use crate::presentation::workspaces::WorkspacePresenter;
use axis_domain::models::airplane::AirplaneStatus;
use axis_domain::models::dnd::DndStatus;
use axis_domain::models::idle_inhibit::IdleInhibitStatus;
use axis_presentation::Presenter;
use std::rc::Rc;
use std::sync::Arc;
pub struct Presenters {
    pub battery: Arc<BatteryPresenter>,
    pub clock: Arc<ClockPresenter>,
    pub workspace: Arc<WorkspacePresenter>,
    pub popup: Arc<PopupPresenter>,
    pub auto_hide: Arc<AutoHidePresenter>,
    pub audio: Rc<AudioPresenter>,
    pub brightness: Rc<BrightnessPresenter>,
    pub agenda: Rc<AgendaPresenter>,
    pub launcher: Rc<LauncherPresenter>,
    pub notification: Rc<NotificationPresenter>,
    pub network: Rc<NetworkPresenter>,
    pub bluetooth: Rc<BluetoothPresenter>,
    pub nightlight: Rc<NightlightPresenter>,
    pub appearance: Rc<AppearancePresenter>,
    pub tray: Rc<TrayPresenter>,
    pub lock: Rc<LockPresenter>,
    pub continuity: Rc<ContinuityPresenter>,
    pub mpris: Rc<MprisPresenter>,
    pub wifi_toggle: Rc<TogglePresenter>,
    pub bluetooth_toggle: Rc<TogglePresenter>,
    pub nightlight_toggle: Rc<TogglePresenter>,
    pub dnd_toggle: Rc<TogglePresenter>,
    pub airplane_toggle: Rc<TogglePresenter>,
    pub continuity_toggle: Rc<TogglePresenter>,
    pub idle_inhibit_toggle: Rc<TogglePresenter>,
    pub dnd_status: Rc<Presenter<DndStatus>>,
    pub idle_inhibit_status: Rc<Presenter<IdleInhibitStatus>>,
    pub airplane_status: Rc<Presenter<AirplaneStatus>>,
}
pub fn setup(_uc: &UseCases, _p: &Providers, _rt: &tokio::runtime::Runtime) -> Presenters {
    unimplemented!("presenters::setup")
}
