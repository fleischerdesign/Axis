use futures_util::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;

pub trait ToggleView {
    fn set_active(&self, active: bool);
    fn set_icon(&self, icon_name: &str);
    fn set_label(&self, label: &str);
    fn on_toggled(&self, f: Box<dyn Fn(bool) + 'static>);
}

pub struct TogglePresenter<T> {
    label: String,
    icon_active: String,
    icon_inactive: String,
    subscribe: Arc<dyn Fn() -> Pin<Box<dyn Stream<Item = bool> + Send>> + Send + Sync>,
    toggle: Arc<dyn Fn(bool) -> T + Send + Sync>,
}

impl<T: 'static> TogglePresenter<T> {
    pub fn new(
        label: &str,
        icon_active: &str,
        icon_inactive: &str,
        subscribe: impl Fn() -> Pin<Box<dyn Stream<Item = bool> + Send>> + Send + Sync + 'static,
        toggle: impl Fn(bool) -> T + Send + Sync + 'static,
    ) -> Self {
        Self {
            label: label.to_string(),
            icon_active: icon_active.to_string(),
            icon_inactive: icon_inactive.to_string(),
            subscribe: Arc::new(subscribe),
            toggle: Arc::new(toggle),
        }
    }

    pub async fn bind(&self, view: Box<dyn ToggleView>) {
        view.set_label(&self.label);

        // UI -> System
        let toggle_fn = self.toggle.clone();
        view.on_toggled(Box::new(move |new_state| {
            let _ = (toggle_fn)(new_state);
        }));

        // System -> UI
        let mut stream = (self.subscribe)();
        while let Some(active) = stream.next().await {
            view.set_active(active);
            view.set_icon(if active { &self.icon_active } else { &self.icon_inactive });
        }
    }
}
