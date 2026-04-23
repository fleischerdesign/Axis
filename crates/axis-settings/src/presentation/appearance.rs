use axis_domain::models::appearance::{AccentColor, AppearanceStatus, ColorScheme};
use axis_presentation::{Presenter, View};
use axis_application::use_cases::appearance::subscribe::SubscribeToAppearanceUseCase;
use axis_application::use_cases::appearance::set_accent::SetAccentColorUseCase;
use axis_application::use_cases::appearance::set_scheme::SetColorSchemeUseCase;
use axis_application::use_cases::appearance::set_wallpaper::SetWallpaperUseCase;
use std::sync::Arc;
use std::rc::Rc;

pub trait AppearanceView: View<AppearanceStatus> {
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
    inner: Presenter<AppearanceStatus>,
    set_accent_uc: Arc<SetAccentColorUseCase>,
    set_scheme_uc: Arc<SetColorSchemeUseCase>,
    set_wallpaper_uc: Arc<SetWallpaperUseCase>,
}

impl AppearancePresenter {
    pub fn new(
        subscribe_uc: Arc<SubscribeToAppearanceUseCase>,
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

    pub fn add_view(&self, view: Box<dyn View<AppearanceStatus>>) {
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
        let uc = self.set_scheme_uc.clone();
        tokio::spawn(async move {
            let _ = uc.execute(scheme).await;
        });
    }

    pub fn set_accent(&self, accent: AccentColor) {
        let uc = self.set_accent_uc.clone();
        tokio::spawn(async move {
            let _ = uc.execute(accent).await;
        });
    }

    pub fn set_wallpaper(&self, path: String) {
        let uc = self.set_wallpaper_uc.clone();
        tokio::spawn(async move {
            let _ = uc.execute(Some(path)).await;
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
