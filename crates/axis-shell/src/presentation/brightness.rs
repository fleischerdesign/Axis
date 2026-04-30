use std::sync::Arc;
use std::rc::Rc;
use std::time::Instant;
use std::cell::RefCell;
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_application::use_cases::brightness::set::SetBrightnessUseCase;
use axis_domain::models::brightness::BrightnessStatus;
use axis_domain::ports::brightness::BrightnessProvider;
use axis_presentation::{Presenter, View};

const FEEDBACK_SUPPRESS_SECS: f64 = 0.5;
const FEEDBACK_TOLERANCE: f64 = 2.0;

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
        let inner = Presenter::from_subscribe({
            let uc = subscribe_use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        });

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
        self.inner.run_with_filter(move |new: &BrightnessStatus, prev: &Option<BrightnessStatus>| {
            if let Some(p) = prev {
                if (p.percentage - new.percentage).abs() < 0.1 {
                    return false;
                }
            }
            let last = last_uc.borrow();
            if let Some((user_val, ts)) = *last {
                if ts.elapsed().as_secs_f64() < FEEDBACK_SUPPRESS_SECS
                    && (new.percentage - user_val).abs() < FEEDBACK_TOLERANCE
                {
                    return false;
                }
            }
            true
        }).await;
    }

    pub fn handle_user_change(&self, new_pct: f64) {
        {
            if let Some(status) = self.inner.current() {
                if (status.percentage - new_pct).abs() < 0.1 { return; }
            }
        }

        *self.last_user_change.borrow_mut() = Some((new_pct, Instant::now()));

        let uc = self.set_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(new_pct).await;
        });
    }
}
