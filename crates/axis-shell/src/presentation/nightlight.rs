use std::sync::Arc;
use axis_application::use_cases::nightlight::subscribe::SubscribeToNightlightUpdatesUseCase;
use axis_application::use_cases::nightlight::get_status::GetNightlightStatusUseCase;
use axis_application::use_cases::nightlight::set_enabled::SetNightlightEnabledUseCase;
use axis_application::use_cases::nightlight::set_temp_day::SetNightlightTempDayUseCase;
use axis_application::use_cases::nightlight::set_temp_night::SetNightlightTempNightUseCase;
use axis_application::use_cases::nightlight::set_schedule::SetNightlightScheduleUseCase;
use axis_domain::models::nightlight::NightlightStatus;
use super::presenter::{Presenter, View};

pub trait NightlightView: View<NightlightStatus> {
    fn on_set_enabled(&self, f: Box<dyn Fn(bool) + 'static>);
    fn on_set_temp_day(&self, f: Box<dyn Fn(u32) + 'static>);
    fn on_set_temp_night(&self, f: Box<dyn Fn(u32) + 'static>);
    fn on_set_schedule(&self, f: Box<dyn Fn(String, String) + 'static>);
}

impl<T: NightlightView + ?Sized> NightlightView for std::rc::Rc<T> {
    fn on_set_enabled(&self, f: Box<dyn Fn(bool) + 'static>) {
        (**self).on_set_enabled(f);
    }
    fn on_set_temp_day(&self, f: Box<dyn Fn(u32) + 'static>) {
        (**self).on_set_temp_day(f);
    }
    fn on_set_temp_night(&self, f: Box<dyn Fn(u32) + 'static>) {
        (**self).on_set_temp_night(f);
    }
    fn on_set_schedule(&self, f: Box<dyn Fn(String, String) + 'static>) {
        (**self).on_set_schedule(f);
    }
}

pub struct NightlightPresenter {
    inner: Presenter<dyn NightlightView, NightlightStatus>,
    set_enabled_use_case: Arc<SetNightlightEnabledUseCase>,
    set_temp_day_use_case: Arc<SetNightlightTempDayUseCase>,
    set_temp_night_use_case: Arc<SetNightlightTempNightUseCase>,
    set_schedule_use_case: Arc<SetNightlightScheduleUseCase>,
}

impl NightlightPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeToNightlightUpdatesUseCase>,
        get_status_use_case: Arc<GetNightlightStatusUseCase>,
        set_enabled_use_case: Arc<SetNightlightEnabledUseCase>,
        set_temp_day_use_case: Arc<SetNightlightTempDayUseCase>,
        set_temp_night_use_case: Arc<SetNightlightTempNightUseCase>,
        set_schedule_use_case: Arc<SetNightlightScheduleUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            get_status_use_case.execute().await.unwrap_or_default()
        });

        let uc = subscribe_use_case.clone();
        let inner = Presenter::new(move || {
            let uc = uc.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = uc.execute().await {
                    while let Some(item) = futures_util::StreamExt::next(&mut stream).await {
                        yield item;
                    }
                }
            })
        }).with_initial_status(initial_status);

        Self {
            inner,
            set_enabled_use_case,
            set_temp_day_use_case,
            set_temp_night_use_case,
            set_schedule_use_case,
        }
    }

    pub fn add_view(&self, view: Box<dyn NightlightView>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run().await;
    }

    pub fn set_enabled(&self, enabled: bool) {
        let uc = self.set_enabled_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(enabled).await;
        });
    }

    pub fn set_temp_day(&self, temp: u32) {
        let uc = self.set_temp_day_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(temp).await;
        });
    }

    pub fn set_temp_night(&self, temp: u32) {
        let uc = self.set_temp_night_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(temp).await;
        });
    }

    pub fn set_schedule(&self, sunrise: String, sunset: String) {
        let uc = self.set_schedule_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(&sunrise, &sunset).await;
        });
    }
}
