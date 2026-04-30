use std::sync::Arc;
use axis_application::use_cases::generic::{GetStatusUseCase, SubscribeUseCase};
use axis_application::use_cases::tray::activate::ActivateTrayItemUseCase;
use axis_application::use_cases::tray::context_menu::ContextMenuTrayItemUseCase;
use axis_application::use_cases::tray::scroll::ScrollTrayItemUseCase;
use axis_domain::models::tray::TrayStatus;
use axis_domain::ports::tray::TrayProvider;
use axis_presentation::{Presenter, View};

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
    inner: Presenter<TrayStatus>,
    activate_use_case: Arc<ActivateTrayItemUseCase>,
    context_menu_use_case: Arc<ContextMenuTrayItemUseCase>,
    scroll_use_case: Arc<ScrollTrayItemUseCase>,
}

impl TrayPresenter {
    pub fn new(
        subscribe_use_case: Arc<SubscribeUseCase<dyn TrayProvider, TrayStatus>>,
        get_status_use_case: Arc<GetStatusUseCase<dyn TrayProvider, TrayStatus>>,
        activate_use_case: Arc<ActivateTrayItemUseCase>,
        context_menu_use_case: Arc<ContextMenuTrayItemUseCase>,
        scroll_use_case: Arc<ScrollTrayItemUseCase>,
        rt: &tokio::runtime::Runtime,
    ) -> Self {
        let initial_status = rt.block_on(async {
            match get_status_use_case.execute().await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[tray] Failed to get initial status: {e}");
                    Default::default()
                }
            }
        });

        let inner = Presenter::from_subscribe({
            let uc = subscribe_use_case.clone();
            move || {
                let uc = uc.clone();
                async move { uc.execute().await }
            }
        })
        .with_initial_status(initial_status);

        Self {
            inner,
            activate_use_case,
            context_menu_use_case,
            scroll_use_case,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<TrayStatus>>) {
        self.inner.add_view(view);
    }

    pub async fn run_sync(&self) {
        self.inner.run_sync().await;
    }

    pub fn activate(&self, bus_name: String, x: i32, y: i32) {
        let uc = self.activate_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&bus_name, x, y).await {
                log::error!("[tray] activate failed: {e}");
            }
        });
    }

    pub fn context_menu(&self, bus_name: String, x: i32, y: i32) {
        let uc = self.context_menu_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&bus_name, x, y).await {
                log::error!("[tray] context_menu failed: {e}");
            }
        });
    }

    pub fn scroll(&self, bus_name: String, delta: i32, orientation: String) {
        let uc = self.scroll_use_case.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(&bus_name, delta, &orientation).await {
                log::error!("[tray] scroll failed: {e}");
            }
        });
    }
}
