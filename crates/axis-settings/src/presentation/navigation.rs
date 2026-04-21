use axis_presentation::{Presenter, View};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PageDescriptor {
    pub id: String,
    pub title: String,
    pub icon: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NavigationState {
    pub pages: Vec<PageDescriptor>,
    pub active_id: String,
}

pub struct NavigationPresenter {
    presenter: Presenter<NavigationState>,
    state_tx: watch::Sender<NavigationState>,
}

pub trait NavigationView: View<NavigationState> {}

impl<T: NavigationView + ?Sized> NavigationView for Rc<T> {}

impl NavigationPresenter {
    pub fn new(initial_pages: Vec<PageDescriptor>) -> Self {
        let active_id = initial_pages.first().map(|p| p.id.clone()).unwrap_or_default();
        let initial_state = NavigationState {
            pages: initial_pages,
            active_id,
        };

        let (state_tx, _) = watch::channel(initial_state);
        let state_tx_c = state_tx.clone();

        let presenter = Presenter::new(move || {
            let rx = state_tx_c.subscribe();
            Box::pin(WatchStream::new(rx))
        });

        Self {
            presenter,
            state_tx,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<NavigationState>>) {
        self.presenter.add_view(view);
    }

    pub async fn run(&self) {
        self.presenter.run_sync().await;
    }

    pub fn select_page(&self, id: &str) {
        let mut state = self.state_tx.borrow().clone();
        if state.pages.iter().any(|p| p.id == id) {
            state.active_id = id.to_string();
            let _ = self.state_tx.send(state);
        }
    }

    pub fn register_page(&self, id: &str, title: &str, icon: &str) {
        let mut state = self.state_tx.borrow().clone();
        if !state.pages.iter().any(|p| p.id == id) {
            state.pages.push(PageDescriptor {
                id: id.to_string(),
                title: title.to_string(),
                icon: icon.to_string(),
            });
            let _ = self.state_tx.send(state);
        }
    }
}
