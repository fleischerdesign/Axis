use axis_application::use_cases::brightness::set::SetBrightnessUseCase;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_domain::models::brightness::BrightnessStatus;
use axis_domain::ports::brightness::BrightnessProvider;
use axis_presentation::{Presenter, View};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

const FEEDBACK_SUPPRESS_SECS: f64 = 0.5;
const FEEDBACK_TOLERANCE: f64 = 0.02;

pub struct BrightnessPresenter {
    inner: Presenter<BrightnessStatus>,
    set_use_case: Arc<SetBrightnessUseCase>,
    last_user_change: Rc<RefCell<Option<(f64, Instant)>>>,
}

impl BrightnessPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn BrightnessProvider, BrightnessStatus>>,
        set_use_case: Arc<SetBrightnessUseCase>,
    ) -> Self {
        let inner = Presenter::from_subscribe_use_case(subscribe_use_case.clone());

        Self {
            inner,
            set_use_case,
            last_user_change: Rc::new(RefCell::new(None)),
        }
    }

    pub fn add_view(&self, view: Box<dyn View<BrightnessStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        let last_uc = self.last_user_change.clone();
        self.inner
            .run_with_filter(
                move |new: &BrightnessStatus, prev: &Option<BrightnessStatus>| {
                    if let Some(p) = prev
                        && (p.percentage - new.percentage).abs() < 0.1
                    {
                        return false;
                    }
                    let last = last_uc.borrow();
                    if let Some((user_val, ts)) = *last
                        && ts.elapsed().as_secs_f64() < FEEDBACK_SUPPRESS_SECS
                        && (new.percentage - user_val).abs() < FEEDBACK_TOLERANCE
                    {
                        return false;
                    }
                    true
                },
            )
            .await;
    }

    pub fn handle_user_change(&self, new_pct: f64) {
        let normalized = new_pct / 100.0;
        {
            if let Some(status) = self.inner.current()
                && (status.percentage - normalized).abs() < 0.01
            {
                return;
            }
        }

        *self.last_user_change.borrow_mut() = Some((normalized, Instant::now()));

        let uc = self.set_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(normalized).await {
                log::error!("[brightness] set_brightness failed: {e}");
            }
        });
    }
}
