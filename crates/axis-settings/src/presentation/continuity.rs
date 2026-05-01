use axis_domain::models::continuity::{ContinuityStatus, PeerArrangement};
use axis_domain::ports::continuity::ContinuityProvider;
use axis_presentation::{Presenter, View};
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::continuity::set_enabled::SetContinuityEnabledUseCase;
use axis_application::use_cases::continuity::connect_to_peer::ConnectToPeerUseCase;
use axis_application::use_cases::continuity::confirm_pin::ConfirmPinUseCase;
use axis_application::use_cases::continuity::reject_pin::RejectPinUseCase;
use axis_application::use_cases::continuity::disconnect::DisconnectUseCase;
use axis_application::use_cases::continuity::unpair::UnpairUseCase;
use axis_application::use_cases::continuity::cancel_reconnect::CancelReconnectUseCase;
use axis_application::use_cases::continuity::set_peer_arrangement::SetPeerArrangementUseCase;
use axis_application::use_cases::continuity::update_peer_configs::UpdatePeerConfigsUseCase;
use std::rc::Rc;
use std::sync::Arc;

pub trait ContinuitySettingsView: View<ContinuityStatus> {
    fn on_toggle_enabled(&self, f: Box<dyn Fn(bool) + 'static>);
    fn on_connect_peer(&self, f: Box<dyn Fn(String) + 'static>);
    fn on_disconnect(&self, f: Box<dyn Fn() + 'static>);
    fn on_confirm_pin(&self, f: Box<dyn Fn() + 'static>);
    fn on_reject_pin(&self, f: Box<dyn Fn() + 'static>);
    fn on_cancel_reconnect(&self, f: Box<dyn Fn() + 'static>);
    fn on_unpair(&self, f: Box<dyn Fn(String) + 'static>);
    fn on_set_arrangement(&self, f: Box<dyn Fn(PeerArrangement) + 'static>);
}

impl<T: ContinuitySettingsView + ?Sized> ContinuitySettingsView for Rc<T> {
    fn on_toggle_enabled(&self, f: Box<dyn Fn(bool) + 'static>) { (**self).on_toggle_enabled(f); }
    fn on_connect_peer(&self, f: Box<dyn Fn(String) + 'static>) { (**self).on_connect_peer(f); }
    fn on_disconnect(&self, f: Box<dyn Fn() + 'static>) { (**self).on_disconnect(f); }
    fn on_confirm_pin(&self, f: Box<dyn Fn() + 'static>) { (**self).on_confirm_pin(f); }
    fn on_reject_pin(&self, f: Box<dyn Fn() + 'static>) { (**self).on_reject_pin(f); }
    fn on_cancel_reconnect(&self, f: Box<dyn Fn() + 'static>) { (**self).on_cancel_reconnect(f); }
    fn on_unpair(&self, f: Box<dyn Fn(String) + 'static>) { (**self).on_unpair(f); }
    fn on_set_arrangement(&self, f: Box<dyn Fn(PeerArrangement) + 'static>) { (**self).on_set_arrangement(f); }
}

pub struct ContinuitySettingsPresenter {
    inner: Presenter<ContinuityStatus>,
    set_enabled_uc: Arc<SetContinuityEnabledUseCase>,
    connect_uc: Arc<ConnectToPeerUseCase>,
    confirm_pin_uc: Arc<ConfirmPinUseCase>,
    reject_pin_uc: Arc<RejectPinUseCase>,
    disconnect_uc: Arc<DisconnectUseCase>,
    cancel_reconnect_uc: Arc<CancelReconnectUseCase>,
    unpair_uc: Arc<UnpairUseCase>,
    set_arrangement_uc: Arc<SetPeerArrangementUseCase>,
    update_configs_uc: Arc<UpdatePeerConfigsUseCase>,
}

impl ContinuitySettingsPresenter {
    pub fn new(
        subscribe_uc: Arc<SubscribeUseCase<dyn ContinuityProvider, ContinuityStatus>>,
        get_status_uc: Arc<GetStatusUseCase<dyn ContinuityProvider, ContinuityStatus>>,
        set_enabled_uc: Arc<SetContinuityEnabledUseCase>,
        connect_uc: Arc<ConnectToPeerUseCase>,
        confirm_pin_uc: Arc<ConfirmPinUseCase>,
        reject_pin_uc: Arc<RejectPinUseCase>,
        disconnect_uc: Arc<DisconnectUseCase>,
        cancel_reconnect_uc: Arc<CancelReconnectUseCase>,
        unpair_uc: Arc<UnpairUseCase>,
        set_arrangement_uc: Arc<SetPeerArrangementUseCase>,
        update_configs_uc: Arc<UpdatePeerConfigsUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            match get_status_uc.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[settings-continuity] Failed to get initial status: {e}");
                    Default::default()
                }
            }
        });

        let sub = subscribe_uc.clone();
        let inner = Presenter::new(move || {
            let uc = sub.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = uc.execute().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status;
                    }
                }
            })
        }).with_initial_status(initial_status);

        Self {
            inner,
            set_enabled_uc,
            connect_uc,
            confirm_pin_uc,
            reject_pin_uc,
            disconnect_uc,
            cancel_reconnect_uc,
            unpair_uc,
            set_arrangement_uc,
            update_configs_uc,
        }
    }

    pub async fn bind(&self, view: Box<dyn ContinuitySettingsView>) {
        let this = self.clone();
        view.on_toggle_enabled(Box::new(move |enabled| {
            let uc = this.set_enabled_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(enabled).await {
                    log::error!("[settings-continuity] set_enabled failed: {e}");
                }
            });
        }));

        let this = self.clone();
        view.on_connect_peer(Box::new(move |id| {
            let uc = this.connect_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(&id).await {
                    log::error!("[settings-continuity] connect failed: {e}");
                }
            });
        }));

        let this = self.clone();
        view.on_disconnect(Box::new(move || {
            let uc = this.disconnect_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute().await {
                    log::error!("[settings-continuity] disconnect failed: {e}");
                }
            });
        }));

        let this = self.clone();
        view.on_confirm_pin(Box::new(move || {
            let uc = this.confirm_pin_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute().await {
                    log::error!("[settings-continuity] confirm_pin failed: {e}");
                }
            });
        }));

        let this = self.clone();
        view.on_reject_pin(Box::new(move || {
            let uc = this.reject_pin_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute().await {
                    log::error!("[settings-continuity] reject_pin failed: {e}");
                }
            });
        }));

        let this = self.clone();
        view.on_cancel_reconnect(Box::new(move || {
            let uc = this.cancel_reconnect_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute().await {
                    log::error!("[settings-continuity] cancel_reconnect failed: {e}");
                }
            });
        }));

        let this = self.clone();
        view.on_unpair(Box::new(move |id| {
            let uc = this.unpair_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(&id).await {
                    log::error!("[settings-continuity] unpair failed: {e}");
                }
            });
        }));

        let this = self.clone();
        view.on_set_arrangement(Box::new(move |arr| {
            let uc = this.set_arrangement_uc.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(arr).await {
                    log::error!("[settings-continuity] set_arrangement failed: {e}");
                }
            });
        }));

        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }
}

impl Clone for ContinuitySettingsPresenter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            set_enabled_uc: self.set_enabled_uc.clone(),
            connect_uc: self.connect_uc.clone(),
            confirm_pin_uc: self.confirm_pin_uc.clone(),
            reject_pin_uc: self.reject_pin_uc.clone(),
            disconnect_uc: self.disconnect_uc.clone(),
            cancel_reconnect_uc: self.cancel_reconnect_uc.clone(),
            unpair_uc: self.unpair_uc.clone(),
            set_arrangement_uc: self.set_arrangement_uc.clone(),
            update_configs_uc: self.update_configs_uc.clone(),
        }
    }
}
