use crate::presentation::appearance::{AppearancePresenter, AppearanceView};
use crate::widgets::callback::FnCell;
use axis_domain::models::appearance::{AccentColor, ColorScheme};
use axis_domain::models::config::AppearanceConfig;
use axis_presentation::View;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct AppearancePage {
    root: adw::ToolbarView,
    scheme_buttons: RefCell<HashMap<ColorScheme, gtk4::Button>>,
    accent_buttons: RefCell<Vec<(AccentColor, gtk4::Button)>>,
    wallpaper_picture: gtk4::Picture,
    wallpaper_title: gtk4::Label,
    wallpaper_subtitle: gtk4::Label,

    scheme_callback: FnCell<ColorScheme>,
    accent_callback: FnCell<AccentColor>,
    wallpaper_callback: FnCell<String>,
}

impl AppearancePage {
    pub fn new(_presenter: Rc<AppearancePresenter>) -> Rc<Self> {
        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header_bar);

        let preferences_page = adw::PreferencesPage::builder()
            .title("Appearance")
            .icon_name("preferences-desktop-wallpaper-symbolic")
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(760)
            .tightening_threshold(500)
            .child(&preferences_page)
            .build();

        toolbar_view.set_content(Some(&clamp));

        // 1. Color Scheme Section
        let scheme_group = adw::PreferencesGroup::builder()
            .title("Color Scheme")
            .description("Choose system appearance theme")
            .build();
        preferences_page.add(&scheme_group);

        // 2. Accent Color Section
        let accent_group = adw::PreferencesGroup::builder()
            .title("Accent Color")
            .description("Select primary highlight color")
            .build();
        preferences_page.add(&accent_group);

        // 3. Wallpaper Section
        let wallpaper_group = adw::PreferencesGroup::builder()
            .title("Wallpaper")
            .description("Desktop background image")
            .build();
        preferences_page.add(&wallpaper_group);

        let wallpaper_picture = gtk4::Picture::builder()
            .content_fit(gtk4::ContentFit::Cover)
            .can_shrink(true)
            .hexpand(true)
            .vexpand(true)
            .halign(gtk4::Align::Fill)
            .valign(gtk4::Align::Fill)
            .css_classes(vec!["wallpaper-picture".to_string()])
            .build();

        let wallpaper_title = gtk4::Label::builder()
            .label("No Wallpaper Set")
            .css_classes(vec!["wallpaper-title".to_string()])
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        let wallpaper_subtitle = gtk4::Label::builder()
            .label("Default background active")
            .css_classes(vec!["wallpaper-subtitle".to_string()])
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        let page = Rc::new(Self {
            root: toolbar_view,
            scheme_buttons: RefCell::new(HashMap::new()),
            accent_buttons: RefCell::new(Vec::new()),
            wallpaper_picture,
            wallpaper_title,
            wallpaper_subtitle,
            scheme_callback: Rc::new(RefCell::new(None)),
            accent_callback: Rc::new(RefCell::new(None)),
            wallpaper_callback: Rc::new(RefCell::new(None)),
        });

        page.setup_scheme_cards(&scheme_group);
        page.setup_accent_swatches(&accent_group);
        page.setup_wallpaper_card(&wallpaper_group);

        page
    }

    fn setup_scheme_cards(&self, group: &adw::PreferencesGroup) {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .homogeneous(true)
            .css_classes(vec!["scheme-box".to_string()])
            .build();

        let schemes = vec![
            (ColorScheme::Dark, "Dark", "scheme-preview-dark"),
            (ColorScheme::Light, "Light", "scheme-preview-light"),
            (ColorScheme::System, "System", "scheme-preview-system"),
        ];

        for (scheme, label_text, preview_class) in schemes {
            let card_content = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(0)
                .build();

            let preview = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .css_classes(vec![
                    "scheme-preview".to_string(),
                    preview_class.to_string(),
                ])
                .build();

            let header = gtk4::Box::builder()
                .css_classes(vec!["scheme-preview-header".to_string()])
                .build();
            let body = gtk4::Box::builder()
                .css_classes(vec!["scheme-preview-body".to_string()])
                .build();

            preview.append(&header);
            preview.append(&body);

            let label = gtk4::Label::builder()
                .label(label_text)
                .halign(gtk4::Align::Center)
                .build();

            card_content.append(&preview);
            card_content.append(&label);

            let button = gtk4::Button::builder()
                .child(&card_content)
                .css_classes(vec!["scheme-card".to_string()])
                .build();

            let cb = self.scheme_callback.clone();
            let scheme_clone = scheme.clone();

            button.connect_clicked(move |_| {
                if let Some(f) = cb.borrow().as_ref() {
                    f(scheme_clone.clone());
                }
            });

            self.scheme_buttons
                .borrow_mut()
                .insert(scheme, button.clone());
            box_.append(&button);
        }

        group.add(&box_);
    }

    fn setup_accent_swatches(&self, group: &adw::PreferencesGroup) {
        let flow_box = gtk4::FlowBox::builder()
            .valign(gtk4::Align::Start)
            .max_children_per_line(10)
            .min_children_per_line(5)
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        // 1. Auto Button first
        let auto_btn = gtk4::Button::builder()
            .tooltip_text("Auto (from wallpaper)")
            .css_classes(vec![
                "accent-swatch".to_string(),
                "accent-auto-swatch".to_string(),
            ])
            .child(&gtk4::Image::from_icon_name(
                "applications-graphics-symbolic",
            ))
            .build();

        let cb_auto = self.accent_callback.clone();
        auto_btn.connect_clicked(move |_| {
            if let Some(f) = cb_auto.borrow().as_ref() {
                f(AccentColor::Auto);
            }
        });

        self.accent_buttons
            .borrow_mut()
            .push((AccentColor::Auto, auto_btn.clone()));
        flow_box.append(&auto_btn);

        // 2. Preset Swatches
        let presets = vec![
            ("Blue", AccentColor::Blue),
            ("Teal", AccentColor::Teal),
            ("Green", AccentColor::Green),
            ("Yellow", AccentColor::Yellow),
            ("Orange", AccentColor::Orange),
            ("Red", AccentColor::Red),
            ("Pink", AccentColor::Pink),
            ("Purple", AccentColor::Purple),
        ];

        for (name, color_type) in presets {
            let hex = color_type.hex_value();
            let btn = gtk4::Button::builder()
                .tooltip_text(name)
                .css_classes(vec!["accent-swatch".to_string()])
                .build();

            let provider = gtk4::CssProvider::new();
            provider.load_from_string(&format!("button {{ background-color: {hex}; }}"));
            #[allow(deprecated)]
            btn.style_context()
                .add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);

            let cb = self.accent_callback.clone();
            let color_c = color_type.clone();

            btn.connect_clicked(move |_| {
                if let Some(f) = cb.borrow().as_ref() {
                    f(color_c.clone());
                }
            });

            self.accent_buttons
                .borrow_mut()
                .push((color_type, btn.clone()));
            flow_box.append(&btn);
        }

        group.add(&flow_box);
    }

    fn setup_wallpaper_card(&self, group: &adw::PreferencesGroup) {
        let card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(vec!["wallpaper-preview-card".to_string()])
            .build();

        let frame = gtk4::Box::builder()
            .css_classes(vec!["wallpaper-picture-frame".to_string()])
            .build();
        frame.append(&self.wallpaper_picture);

        let bottom_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .valign(gtk4::Align::Center)
            .build();

        let label_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .hexpand(true)
            .build();
        label_box.append(&self.wallpaper_title);
        label_box.append(&self.wallpaper_subtitle);

        let select_btn = gtk4::Button::builder()
            .label("Select File...")
            .valign(gtk4::Align::Center)
            .build();

        bottom_row.append(&label_box);
        bottom_row.append(&select_btn);

        card.append(&frame);
        card.append(&bottom_row);

        let cb = self.wallpaper_callback.clone();
        let btn_c = select_btn.clone();
        select_btn.connect_clicked(move |_| {
            let dialog = gtk4::FileDialog::builder()
                .title("Select Wallpaper")
                .build();

            let filter = gtk4::FileFilter::new();
            filter.add_pixbuf_formats();
            filter.set_name(Some("Images"));
            let filters = gtk4::gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));

            let cb_inner = cb.clone();
            dialog.open(
                btn_c.root().and_downcast_ref::<gtk4::Window>(),
                None::<&gtk4::gio::Cancellable>,
                move |result| {
                    if let Ok(file) = result
                        && let Some(path) = file.path()
                        && let Some(path_str) = path.to_str()
                        && let Some(f) = cb_inner.borrow().as_ref()
                    {
                        f(path_str.to_string());
                    }
                },
            );
        });

        group.add(&card);
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }
}

impl View<AppearanceConfig> for AppearancePage {
    fn render(&self, status: &AppearanceConfig) {
        // 1. Color Scheme Active State
        for (scheme, btn) in self.scheme_buttons.borrow().iter() {
            if status.color_scheme == *scheme {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }

        // 2. Accent Color Active State
        for (color, btn) in self.accent_buttons.borrow().iter() {
            if status.accent_color == *color {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }

        // 3. Wallpaper Preview & Labels
        if let Some(ref path_str) = status.wallpaper {
            let path = std::path::Path::new(path_str);
            if path.exists() {
                self.wallpaper_picture.set_filename(Some(path_str));
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path_str);
                let parent = path.parent().and_then(|p| p.to_str()).unwrap_or("");
                self.wallpaper_title.set_text(filename);
                self.wallpaper_subtitle.set_text(parent);
            } else {
                self.wallpaper_picture.set_filename(None::<&str>);
                self.wallpaper_title.set_text("File not found");
                self.wallpaper_subtitle.set_text(path_str);
            }
        } else {
            self.wallpaper_picture.set_filename(None::<&str>);
            self.wallpaper_title.set_text("No Wallpaper Set");
            self.wallpaper_subtitle
                .set_text("Default background active");
        }
    }
}

impl AppearanceView for AppearancePage {
    fn on_scheme_changed(&self, f: Box<dyn Fn(ColorScheme) + 'static>) {
        *self.scheme_callback.borrow_mut() = Some(f);
    }
    fn on_accent_changed(&self, f: Box<dyn Fn(AccentColor) + 'static>) {
        *self.accent_callback.borrow_mut() = Some(f);
    }
    fn on_wallpaper_selected(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.wallpaper_callback.borrow_mut() = Some(f);
    }
}
