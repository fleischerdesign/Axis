use std::sync::Arc;
use axis_application::use_cases::tray::subscribe::SubscribeToTrayUpdatesUseCase;
use axis_application::use_cases::tray::get_status::GetTrayStatusUseCase;
use axis_application::use_cases::tray::activate::ActivateTrayItemUseCase;
use axis_application::use_cases::tray::context_menu::ContextMenuTrayItemUseCase;
use axis_application::use_cases::tray::scroll::ScrollTrayItemUseCase;
use axis_domain::models::tray::TrayStatus;
use super::presenter::{Presenter, View};

pub trait TrayView: View<TrayStatus> {
    fn on_activate(&self, f: Box<dyn Fn(String, i32, i32) + 'static>);
    fn on_context_menu(&self, f: Box<dyn Fn(String, i32, i32) + 'static>);
    fn on_scroll(&self, f: Box<dyn Fn(String, i32, String) + 'static>);
}

impl<T: TrayView + ?Sized> TrayView for std::rc::Rc<T> {
    fn on_activate(&self, f: Box<dyn Fn(String, i32, i32) + 'static>) {
        (**self).on_activate(f);
    }
    fn on_context_menu(&self, f: Box<dyn Fn(String, i32, i32) + 'static>) {
        (**self).on_context_menu(f);
    }
    fn on_scroll(&self, f: Box<dyn Fn(String, i32, String) + 'static>) {
        (**self).on_scroll(f);
    }
}

pub struct TrayPresenter {
    inner: Presenter<dyn TrayView, TrayStatus>,
    activate_use_case: Arc<ActivateTrayItemUseCase>,
    context_menu_use_case: Arc<ContextMenuTrayItemUseCase>,
    scroll_use_case: Arc<ScrollTrayItemUseCase>,
}

impl TrayPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeToTrayUpdatesUseCase>,
        get_status_use_case: Arc<GetTrayStatusUseCase>,
        activate_use_case: Arc<ActivateTrayItemUseCase>,
        context_menu_use_case: Arc<ContextMenuTrayItemUseCase>,
        scroll_use_case: Arc<ScrollTrayItemUseCase>,
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
        })
        .with_initial_status(initial_status);

        Self {
            inner,
            activate_use_case,
            context_menu_use_case,
            scroll_use_case,
        }
    }

    pub fn add_view(&self, view: Box<dyn TrayView>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run().await;
    }

    pub fn activate(&self, bus_name: String, x: i32, y: i32) {
        let uc = self.activate_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(&bus_name, x, y).await;
        });
    }

    pub fn context_menu(&self, bus_name: String, x: i32, y: i32) {
        let uc = self.context_menu_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(&bus_name, x, y).await;
        });
    }

    pub fn scroll(&self, bus_name: String, delta: i32, orientation: String) {
        let uc = self.scroll_use_case.clone();
        tokio::spawn(async move {
            let _ = uc.execute(&bus_name, delta, &orientation).await;
        });
    }
}
