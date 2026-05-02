use axis_domain::models::config::AxisConfig;
use axis_application::use_cases::idle_inhibit::set_inhibited::SetIdleInhibitUseCase;
use axis_domain::ports::config::ConfigProvider;
use axis_presentation::{Presenter, View};
use std::rc::Rc;
use std::sync::Arc;
use gtk4::glib;

pub trait IdleSettingsView: View<AxisConfig> {
    fn on_inhibited_toggled(&self, f: Box<dyn Fn(bool) + 'static>);
    fn on_lock_timeout_changed(&self, f: Box<dyn Fn(Option<u32>) + 'static>);
    fn on_blank_timeout_changed(&self, f: Box<dyn Fn(Option<u32>) + 'static>);
}

impl<T: IdleSettingsView + ?Sized> IdleSettingsView for Rc<T> {
    fn on_inhibited_toggled(&self, f: Box<dyn Fn(bool) + 'static>) {
        (**self).on_inhibited_toggled(f);
    }
    fn on_lock_timeout_changed(&self, f: Box<dyn Fn(Option<u32>) + 'static>) {
        (**self).on_lock_timeout_changed(f);
    }
    fn on_blank_timeout_changed(&self, f: Box<dyn Fn(Option<u32>) + 'static>) {
        (**self).on_blank_timeout_changed(f);
    }
}

pub struct IdleSettingsPresenter {
    inner: Presenter<AxisConfig>,
    set_inhibited_uc: Arc<SetIdleInhibitUseCase>,
    config_provider: Arc<dyn ConfigProvider>,
}

impl Clone for IdleSettingsPresenter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            set_inhibited_uc: self.set_inhibited_uc.clone(),
            config_provider: self.config_provider.clone(),
        }
    }
}

impl IdleSettingsPresenter {
    pub fn new(
        config_provider: Arc<dyn ConfigProvider>,
        set_inhibited_uc: Arc<SetIdleInhibitUseCase>,
    ) -> Self {
        let cp = config_provider.clone();
        let inner = Presenter::new(move || {
            let cp = cp.clone();
            Box::pin(async_stream::stream! {
                match cp.subscribe() {
                    Ok(mut stream) => {
                        while let Some(config) = futures_util::StreamExt::next(&mut stream).await {
                            yield config;
                        }
                    }
                    Err(e) => {
                        log::error!("[settings-idle] config subscribe failed: {e}");
                    }
                }
            })
        });
        Self {
            inner,
            set_inhibited_uc,
            config_provider,
        }
    }

    pub async fn bind(&self, view: Box<dyn IdleSettingsView>) {
        let this = self.clone();
        view.on_inhibited_toggled(Box::new(move |inhibited| {
            let this = this.clone();
            glib::spawn_future_local(async move {
                if let Err(e) = this.set_inhibited_uc.execute(inhibited).await {
                    log::error!("[settings-idle] set_inhibited failed: {e}");
                }
            });
        }));

        let cp_lock = self.config_provider.clone();
        view.on_lock_timeout_changed(Box::new(move |timeout| {
            let cp = cp_lock.clone();
            if let Err(e) = cp.update(Box::new(move |cfg| {
                cfg.idle.lock_timeout_seconds = timeout;
            })) {
                log::error!("[settings-idle] set lock timeout failed: {e}");
            }
        }));

        let cp_blank = self.config_provider.clone();
        view.on_blank_timeout_changed(Box::new(move |timeout| {
            let cp = cp_blank.clone();
            if let Err(e) = cp.update(Box::new(move |cfg| {
                cfg.idle.blank_timeout_seconds = timeout;
            })) {
                log::error!("[settings-idle] set blank timeout failed: {e}");
            }
        }));

        self.inner.add_view(view);
    }

    pub async fn run(&self) {
        self.inner.run_sync().await;
    }
}
