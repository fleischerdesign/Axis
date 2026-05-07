use axis_application::use_cases::bluetooth::pair_accept::PairAcceptUseCase;
use axis_application::use_cases::bluetooth::pair_reject::PairRejectUseCase;
use axis_application::use_cases::continuity::confirm_pin::ConfirmPinUseCase;
use axis_application::use_cases::continuity::reject_pin::RejectPinUseCase;
use axis_application::use_cases::notifications::show_notification::ShowNotificationUseCase;
use axis_domain::ports::bluetooth::BluetoothProvider;
use axis_domain::ports::config::ConfigProvider;
use axis_domain::ports::continuity::ContinuityProvider;
use std::sync::Arc;
pub fn subscribe_continuity_notifications(
    _cp: Arc<dyn ContinuityProvider>,
    _sn: Arc<ShowNotificationUseCase>,
    _cp2: Arc<ConfirmPinUseCase>,
    _rp: Arc<RejectPinUseCase>,
    _rt: &tokio::runtime::Runtime,
) {
}
pub fn subscribe_bluetooth_pairing_notifications(
    _bp: Arc<dyn BluetoothProvider>,
    _sn: Arc<ShowNotificationUseCase>,
    _pa: Arc<PairAcceptUseCase>,
    _pr: Arc<PairRejectUseCase>,
) {
}
pub fn wire_continuity_sync(
    _cfg: Arc<dyn ConfigProvider>,
    _cp: Arc<dyn ContinuityProvider>,
    _rt: &tokio::runtime::Runtime,
) {
}
