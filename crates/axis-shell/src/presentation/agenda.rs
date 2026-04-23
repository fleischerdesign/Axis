use std::sync::Arc;
use axis_domain::models::agenda::AgendaStatus;
use axis_presentation::{Presenter, View};

pub trait AgendaView: View<AgendaStatus> {}
impl<T: View<AgendaStatus> + ?Sized> AgendaView for T {}

pub struct AgendaPresenter {
    inner: Presenter<AgendaStatus>,
}

impl AgendaPresenter {
    pub fn new() -> Self {
        let inner = Presenter::new(|| {
            // Initial empty stream until we have real providers
            Box::pin(futures_util::stream::pending())
        }).with_initial_status(AgendaStatus::default());

        Self { inner }
    }

    pub fn add_view(&self, view: Box<dyn View<AgendaStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn update_status(&self, status: AgendaStatus) {
        self.inner.update(status);
    }
}
