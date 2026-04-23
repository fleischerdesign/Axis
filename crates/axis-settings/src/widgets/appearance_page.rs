use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::cell::RefCell;
use axis_domain::models::appearance::{AccentColor, AppearanceStatus, ColorScheme};
use crate::presentation::appearance::{AppearanceView, AppearancePresenter};
use axis_presentation::View;

pub struct AppearancePage {
    root: adw::ToolbarView,
    scheme_group: adw::PreferencesGroup,
    accent_group: adw::PreferencesGroup,
    wallpaper_group: adw::PreferencesGroup,
    wallpaper_row: adw::ActionRow,
    
    scheme_callback: Rc<RefCell<Option<Box<dyn Fn(ColorScheme) + 'static>>>>,
    accent_callback: Rc<RefCell<Option<Box<dyn Fn(AccentColor) + 'static>>>>,
    wallpaper_callback: Rc<RefCell<Option<Box<dyn Fn(String) + 'static>>>>,
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
        toolbar_view.set_content(Some(&preferences_page));

        // 1. Color Scheme Section
        let scheme_group = adw::PreferencesGroup::builder()
            .title("Color Scheme")
            .build();
        preferences_page.add(&scheme_group);

        // 2. Accent Color Section
        let accent_group = adw::PreferencesGroup::builder()
            .title("Accent Color")
            .build();
        preferences_page.add(&accent_group);

        // 3. Wallpaper Section
        let wallpaper_group = adw::PreferencesGroup::builder()
            .title("Wallpaper")
            .build();
        preferences_page.add(&wallpaper_group);

        let wallpaper_row = adw::ActionRow::builder()
            .title("Background Image")
            .subtitle("Select a wallpaper")
            .build();
        
        let select_btn = gtk4::Button::builder()
            .label("Select File...")
            .valign(gtk4::Align::Center)
            .build();
        wallpaper_row.add_suffix(&select_btn);
        wallpaper_group.add(&wallpaper_row);

        let page = Rc::new(Self {
            root: toolbar_view,
            scheme_group,
            accent_group,
            wallpaper_group,
            wallpaper_row,
            scheme_callback: Rc::new(RefCell::new(None)),
            accent_callback: Rc::new(RefCell::new(None)),
            wallpaper_callback: Rc::new(RefCell::new(None)),
        });

        page.setup_scheme_row();
        page.setup_accent_row();
        page.setup_wallpaper_logic(&select_btn);

        page
    }

    fn setup_scheme_row(&self) {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(0)
            .css_classes(vec!["linked".to_string()])
            .halign(gtk4::Align::Center)
            .build();

        let dark_btn = gtk4::ToggleButton::builder().label("Dark").build();
        let light_btn = gtk4::ToggleButton::builder().label("Light").group(&dark_btn).build();

        box_.append(&dark_btn);
        box_.append(&light_btn);
        
        self.scheme_group.add(&box_);

        let cb_d = self.scheme_callback.clone();
        dark_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                if let Some(f) = cb_d.borrow().as_ref() { f(ColorScheme::Dark); }
            }
        });
        
        let cb_l = self.scheme_callback.clone();
        light_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                if let Some(f) = cb_l.borrow().as_ref() { f(ColorScheme::Light); }
            }
        });
    }

    fn setup_accent_row(&self) {
        let flow_box = gtk4::FlowBox::builder()
            .valign(gtk4::Align::Start)
            .max_children_per_line(10)
            .min_children_per_line(5)
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let colors = vec![
            ("Blue", AccentColor::Blue, "#3584e4"),
            ("Teal", AccentColor::Teal, "#21c7c7"),
            ("Green", AccentColor::Green, "#3ec35a"),
            ("Yellow", AccentColor::Yellow, "#f9f06b"),
            ("Orange", AccentColor::Orange, "#ff9500"),
            ("Red", AccentColor::Red, "#f66151"),
            ("Pink", AccentColor::Pink, "#d56191"),
            ("Purple", AccentColor::Purple, "#9141ac"),
        ];

        for (name, color_type, hex) in colors {
            let btn = gtk4::Button::builder()
                .tooltip_text(name)
                .css_classes(vec!["accent-button".to_string()])
                .build();
            
            // Set individual button color
            let provider = gtk4::CssProvider::new();
            provider.load_from_string(&format!("button {{ background-color: {}; }}", hex));
            #[allow(deprecated)]
            btn.style_context().add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
let cb = self.accent_callback.clone();
let color_c = color_type.clone();

btn.connect_clicked(move |_| {
    if let Some(f) = cb.borrow().as_ref() { f(color_c.clone()); }
});


            flow_box.append(&btn);
        }

        self.accent_group.add(&flow_box);
    }

    fn setup_wallpaper_logic(&self, btn: &gtk4::Button) {
        let btn_c = btn.clone();
        let cb = self.wallpaper_callback.clone();
        btn.connect_clicked(move |_| {
            let chooser = gtk4::FileChooserNative::new(
                Some("Select Wallpaper"),
                btn_c.root().and_downcast_ref::<gtk4::Window>(),
                gtk4::FileChooserAction::Open,
                Some("Select"),
                Some("Cancel"),
            );

            let filter = gtk4::FileFilter::new();
            filter.add_pixbuf_formats();
            filter.set_name(Some("Images"));
            chooser.add_filter(&filter);

            let cb_inner = cb.clone();
            chooser.connect_response(move |dialog, response| {
                if response == gtk4::ResponseType::Accept {
                    if let Some(file) = dialog.file() {
                        if let Some(path) = file.path() {
                            if let Some(path_str) = path.to_str() {
                                if let Some(f) = cb_inner.borrow().as_ref() {
                                    f(path_str.to_string());
                                }
                            }
                        }
                    }
                }
                dialog.destroy();
            });

            chooser.show();
        });
    }

    pub fn widget(&self) -> &adw::ToolbarView {
        &self.root
    }
}

impl View<AppearanceStatus> for AppearancePage {
    fn render(&self, status: &AppearanceStatus) {
        if let Some(ref path) = status.wallpaper {
            self.wallpaper_row.set_subtitle(path);
        } else {
            self.wallpaper_row.set_subtitle("None");
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
