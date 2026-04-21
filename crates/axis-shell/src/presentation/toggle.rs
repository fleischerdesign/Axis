use std::pin::Pin;
use std::sync::Arc;
use futures_util::Stream;
use axis_presentation::{Presenter, View, view::FnView};

pub trait ToggleView: View<bool> {
    fn set_label(&self, label: &str);
    fn set_icon(&self, icon_name: &str);
    fn on_toggled(&self, f: Box<dyn Fn(bool) + 'static>);
}

pub struct TogglePresenter<T> {
    inner: Presenter<bool>,
    label: String,
    icon_active: String,
    icon_inactive: String,
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
        let inner = Presenter::new(subscribe);
        Self {
            inner,
            label: label.to_string(),
            icon_active: icon_active.to_string(),
            icon_inactive: icon_inactive.to_string(),
            toggle: Arc::new(toggle),
        }
    }

    pub async fn bind(&self, view: Box<dyn ToggleView>) {
        view.set_label(&self.label);
        
        let toggle_fn = self.toggle.clone();
        view.on_toggled(Box::new(move |new_state| {
            let _ = (toggle_fn)(new_state);
        }));

        let icon_active = self.icon_active.clone();
        let icon_inactive = self.icon_inactive.clone();
        
        // Wrap the view in an Arc to share it between the FnView and the Presenter
        let view_shared: Arc<dyn ToggleView> = Arc::from(view);
        let view_c = view_shared.clone();

        self.inner.add_view(Box::new(FnView::new(move |active: &bool| {
            view_c.render(active);
            view_c.set_icon(if *active { &icon_active } else { &icon_inactive });
        })));

        self.inner.run_sync().await;
    }
}
