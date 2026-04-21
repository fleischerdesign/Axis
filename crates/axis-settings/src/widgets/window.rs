use libadwaita::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::collections::HashMap;
use std::cell::RefCell;
use crate::presentation::navigation::{NavigationState, NavigationView};
use axis_presentation::View;

pub struct SettingsWindow {
    window: adw::ApplicationWindow,
    nav_view: adw::NavigationView,
    pages: RefCell<HashMap<String, adw::NavigationPage>>,
}

impl SettingsWindow {
    pub fn new(app: &adw::Application, sidebar: &gtk4::Widget) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(900)
            .default_height(650)
            .title("Settings")
            .build();

        let nav_view = adw::NavigationView::new();
        
        let sidebar_scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(sidebar)
            .build();

        let sidebar_page = adw::NavigationPage::new(&sidebar_scrolled, "Settings");
        let content_page = adw::NavigationPage::new(&nav_view, "Content");

        let split_view = adw::NavigationSplitView::builder()
            .sidebar(&sidebar_page)
            .content(&content_page)
            .build();

        window.set_content(Some(&split_view));

        Rc::new(Self {
            window,
            nav_view,
            pages: RefCell::new(HashMap::new()),
        })
    }

    pub fn register_page_widget(&self, id: &str, title: &str, widget: &impl IsA<gtk4::Widget>) {
        let page = adw::NavigationPage::new(widget, title);
        page.set_tag(Some(id));
        self.pages.borrow_mut().insert(id.to_string(), page);
    }

    pub fn present(&self) {
        self.window.present();
    }
}

impl View<NavigationState> for SettingsWindow {
    fn render(&self, state: &NavigationState) {
        if let Some(page) = self.pages.borrow().get(&state.active_id) {
            let current_tag = self.nav_view.visible_page()
                .and_then(|p| p.tag().map(|t| t.to_string()));
            
            if current_tag.as_deref() != Some(&state.active_id) {
                self.nav_view.replace(&[page.clone()]);
            }
        }
    }
}

impl NavigationView for SettingsWindow {}
