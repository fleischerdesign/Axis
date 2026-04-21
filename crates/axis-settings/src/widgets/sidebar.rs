use libadwaita::prelude::*;
use libadwaita as adw;
use gtk4::glib;
use std::rc::Rc;
use crate::presentation::navigation::{NavigationState, NavigationView, NavigationPresenter};
use axis_presentation::View;

pub struct Sidebar {
    list_box: gtk4::ListBox,
    presenter: Rc<NavigationPresenter>,
}

impl Sidebar {
    pub fn new(presenter: Rc<NavigationPresenter>) -> Rc<Self> {
        let list_box = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(vec!["sidebar-list".to_string()])
            .build();

        let p_c = presenter.clone();
        list_box.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let tag = row.widget_name();
                p_c.select_page(&tag);
            }
        });

        Rc::new(Self {
            list_box,
            presenter,
        })
    }

    pub fn widget(&self) -> &gtk4::ListBox {
        &self.list_box
    }

    fn create_row(&self, title: &str, icon: &str, tag: &str) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(title)
            .activatable(true)
            .build();
        row.set_widget_name(tag);
        let img = gtk4::Image::from_icon_name(icon);
        row.add_prefix(&img);
        row
    }
}

impl View<NavigationState> for Sidebar {
    fn render(&self, state: &NavigationState) {
        // Clear existing rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        for page in &state.pages {
            let row = self.create_row(&page.title, &page.icon, &page.id);
            self.list_box.append(&row);

            if page.id == state.active_id {
                self.list_box.select_row(Some(&row));
            }
        }
    }
}

impl NavigationView for Sidebar {}
