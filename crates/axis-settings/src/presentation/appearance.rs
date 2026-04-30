use axis_domain::models::appearance::{AccentColor, ColorScheme};
use axis_domain::models::config::AppearanceConfig;
use axis_domain::ports::appearance::AppearanceProvider;
use axis_presentation::{Presenter, View};
use axis_application::use_cases::generic::SubscribeUseCase;
use axis_application::use_cases::appearance::set_accent::SetAccentColorUseCase;
use axis_application::use_cases::appearance::set_scheme::SetColorSchemeUseCase;
use axis_application::use_cases::appearance::set_wallpaper::SetWallpaperUseCase;
use std::sync::Arc;
use std::rc::Rc;

pub trait AppearanceView: View<AppearanceConfig> {
    fn on_scheme_changed(&self, f: Box<dyn Fn(ColorScheme) + 'static>);
    fn on_accent_changed(&self, f: Box<dyn Fn(AccentColor) + 'static>);
    fn on_wallpaper_selected(&self, f: Box<dyn Fn(String) + 'static>);
}

impl<T: AppearanceView + ?Sized> AppearanceView for Rc<T> {
    fn on_scheme_changed(&self, f: Box<dyn Fn(ColorScheme) + 'static>) {
        (**self).on_scheme_changed(f);
    }
    fn on_accent_changed(&self, f: Box<dyn Fn(AccentColor) + 'static>) {
        (**self).on_accent_changed(f);
    }
    fn on_wallpaper_selected(&self, f: Box<dyn Fn(String) + 'static>) {
        (**self).on_wallpaper_selected(f);
    }
}

pub struct AppearancePresenter {
    inner: Presenter<AppearanceConfig>,
    set_accent_uc: Arc<SetAccentColorUseCase>,
    set_scheme_uc: Arc<SetColorSchemeUseCase>,
    set_wallpaper_uc: Arc<SetWallpaperUseCase>,
}

impl AppearancePresenter {
    pub fn new(
        subscribe_uc: Arc<SubscribeUseCase<dyn AppearanceProvider, AppearanceConfig>>,
        set_accent_uc: Arc<SetAccentColorUseCase>,
        set_scheme_uc: Arc<SetColorSchemeUseCase>,
        set_wallpaper_uc: Arc<SetWallpaperUseCase>,
    ) -> Self {
        let sub = subscribe_uc.clone();
        let inner = Presenter::new(move || {
            let sub = sub.clone();
            Box::pin(async_stream::stream! {
                if let Ok(mut stream) = sub.execute().await {
                    while let Some(status) = futures_util::StreamExt::next(&mut stream).await {
                        yield status;
                    }
                }
            })
        });

        Self {
            inner,
            set_accent_uc,
            set_scheme_uc,
            set_wallpaper_uc,
        }
    }

    pub fn add_view(&self, view: Box<dyn View<AppearanceConfig>>) {
        self.inner.add_view(view);
    }

    pub async fn bind(&self, view: Box<dyn AppearanceView>) {
        let this = self.clone();
        view.on_scheme_changed(Box::new(move |scheme| {
            this.set_scheme(scheme);
        }));

        let this_accent = self.clone();
        view.on_accent_changed(Box::new(move |accent| {
            this_accent.set_accent(accent);
        }));

        let this_wp = self.clone();
        view.on_wallpaper_selected(Box::new(move |path| {
            this_wp.set_wallpaper(path);
        }));

        self.inner.add_view(view);
    }

    pub async fn run(&self) {
        self.inner.run_sync().await;
    }

    pub fn set_scheme(&self, scheme: ColorScheme) {
        // Optimistic UI Update: Use inner.current()
        let mut status = self.inner.current().unwrap_or_default();
        status.color_scheme = scheme.clone();
        self.inner.update(status);

        let uc = self.set_scheme_uc.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(scheme).await {
                log::error!("[settings-appearance] set_scheme failed: {e}");
            }
        });
    }

    pub fn set_accent(&self, accent: AccentColor) {
        let mut status = self.inner.current().unwrap_or_default();
        status.accent_color = accent.clone();
        self.inner.update(status);

        let uc = self.set_accent_uc.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(accent).await {
                log::error!("[settings-appearance] set_accent failed: {e}");
            }
        });
    }

    pub fn set_wallpaper(&self, path: String) {
        let mut status = self.inner.current().unwrap_or_default();
        status.wallpaper = Some(path.clone());
        self.inner.update(status);

        let uc = self.set_wallpaper_uc.clone();
        tokio::spawn(async move {
            if let Err(e) = uc.execute(path).await {
                log::error!("[settings-appearance] set_wallpaper failed: {e}");
            }
        });
    }

}

impl Clone for AppearancePresenter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            set_accent_uc: self.set_accent_uc.clone(),
            set_scheme_uc: self.set_scheme_uc.clone(),
            set_wallpaper_uc: self.set_wallpaper_uc.clone(),
        }
    }
}
