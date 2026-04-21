use std::sync::Arc;
use axis_application::use_cases::clock::subscribe::SubscribeToClockUpdatesUseCase;
use axis_domain::models::clock::TimeStatus;
use super::presenter::{Presenter, View};

pub trait ClockView: View<TimeStatus> {}

pub struct ClockPresenter {
    inner: Presenter<dyn ClockView, TimeStatus>,
}

impl ClockPresenter {
    pub fn new(use_case: Arc<SubscribeToClockUpdatesUseCase>) -> Self {
        let uc = use_case.clone();
        let inner = Presenter::new(move || {
            let uc = uc.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = uc.execute().await {
                    while let Some(item) = futures_util::StreamExt::next(&mut stream).await {
                        yield item;
                    }
                }
            })
        });
        Self { inner }
    }

    pub async fn bind(&self, view: Box<dyn ClockView>) {
        self.inner.bind(view).await;
    }
}
