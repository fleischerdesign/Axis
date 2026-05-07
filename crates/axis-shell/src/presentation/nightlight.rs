use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::nightlight::set_enabled::SetNightlightEnabledUseCase;
use axis_application::use_cases::nightlight::set_schedule::SetNightlightScheduleUseCase;
use axis_application::use_cases::nightlight::set_temp_day::SetNightlightTempDayUseCase;
use axis_application::use_cases::nightlight::set_temp_night::SetNightlightTempNightUseCase;
use axis_domain::models::nightlight::NightlightStatus;
use axis_domain::ports::nightlight::NightlightProvider;
use axis_presentation::{Presenter, View};
use std::sync::Arc;

pub struct NightlightPresenter {
    inner: Presenter<NightlightStatus>,
    set_enabled_use_case: Arc<SetNightlightEnabledUseCase>,
    set_temp_day_use_case: Arc<SetNightlightTempDayUseCase>,
    set_temp_night_use_case: Arc<SetNightlightTempNightUseCase>,
    set_schedule_use_case: Arc<SetNightlightScheduleUseCase>,
}

pub struct NightlightPresenterArgs {
    pub subscribe_uc: Arc<SubscribeUseCase<dyn NightlightProvider, NightlightStatus>>,
    pub get_status_uc: Arc<GetStatusUseCase<dyn NightlightProvider, NightlightStatus>>,
    pub set_enabled_uc: Arc<SetNightlightEnabledUseCase>,
    pub set_temp_day_uc: Arc<SetNightlightTempDayUseCase>,
    pub set_temp_night_uc: Arc<SetNightlightTempNightUseCase>,
    pub set_schedule_uc: Arc<SetNightlightScheduleUseCase>,
}

impl NightlightPresenter {
    pub fn new(args: NightlightPresenterArgs, rt: &tokio::runtime::Runtime) -> Self {
        let NightlightPresenterArgs {
            subscribe_uc,
            get_status_uc,
            set_enabled_uc,
            set_temp_day_uc,
            set_temp_night_uc,
            set_schedule_uc,
        } = args;

        let initial_status = rt.block_on(async {
            match get_status_uc.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[nightlight] Failed to get initial status: {e}");
                    Default::default()
                }
            }
        });

        let inner = Presenter::from_subscribe_use_case(subscribe_uc.clone())
            .with_initial_status(initial_status);

        Self {
            inner,
            set_enabled_use_case: set_enabled_uc,
            set_temp_day_use_case: set_temp_day_uc,
            set_temp_night_use_case: set_temp_night_uc,
            set_schedule_use_case: set_schedule_uc,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<NightlightStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn set_enabled(&self, enabled: bool) {
        let uc = self.set_enabled_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(enabled).await {
                log::error!("[nightlight] set_enabled failed: {e}");
            }
        });
    }

    pub fn set_temp_day(&self, temp: u32) {
        let uc = self.set_temp_day_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(temp).await {
                log::error!("[nightlight] set_temp_day failed: {e}");
            }
        });
    }

    pub fn set_temp_night(&self, temp: u32) {
        let uc = self.set_temp_night_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(temp).await {
                log::error!("[nightlight] set_temp_night failed: {e}");
            }
        });
    }

    pub fn set_schedule(&self, sunrise: String, sunset: String) {
        let uc = self.set_schedule_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&sunrise, &sunset).await {
                log::error!("[nightlight] set_schedule failed: {e}");
            }
        });
    }
}
