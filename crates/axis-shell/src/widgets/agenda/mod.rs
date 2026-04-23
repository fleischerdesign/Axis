use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::{glib, gio};
use axis_domain::models::popups::{PopupType, PopupStatus};
use axis_domain::models::agenda::AgendaStatus;
use crate::widgets::popup_base::PopupContainer;
use crate::presentation::popups::PopupView;
use crate::presentation::agenda::AgendaView;
use axis_presentation::View;
use std::rc::Rc;

mod calendar_grid;
mod task_list;

use calendar_grid::CalendarGrid;
use task_list::TaskList;

glib::wrapper! {
    pub struct AgendaPopup(ObjectSubclass<imp::AgendaPopup>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl AgendaPopup {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }
}

impl View<PopupStatus> for AgendaPopup {
    fn render(&self, status: &PopupStatus) {
        self.handle_status(status);
    }
}

impl PopupView for AgendaPopup {
    fn get_type(&self) -> PopupType { PopupType::Agenda }
    fn popup_container(&self) -> PopupContainer { self.imp().container.clone() }
    fn popup_window(&self) -> gtk4::ApplicationWindow { self.clone().upcast() }
}

impl View<AgendaStatus> for AgendaPopup {
    fn render(&self, status: &AgendaStatus) {
        self.imp().calendar_grid.render(status);
        self.imp().task_list.render(status);
    }
}

impl AgendaView for AgendaPopup {
    fn on_list_changed(&self, f: Box<dyn Fn(String) + 'static>) {
        self.imp().task_list.on_list_changed(f);
    }

    fn on_task_toggled(&self, f: Box<dyn Fn(String, bool) + 'static>) {
        self.imp().task_list.on_task_toggled(f);
    }

    fn on_task_deleted(&self, f: Box<dyn Fn(String) + 'static>) {
        self.imp().task_list.on_task_deleted(f);
    }

    fn on_task_created(&self, f: Box<dyn Fn(String) + 'static>) {
        self.imp().task_list.on_task_created(f);
    }
}

mod imp {
    use super::*;
    use gtk4_layer_shell::{LayerShell, Layer, Edge, KeyboardMode};

    pub struct AgendaPopup {
        pub container: PopupContainer,
        pub calendar_grid: CalendarGrid,
        pub task_list: TaskList,
    }

    impl Default for AgendaPopup {
        fn default() -> Self {
            Self {
                container: PopupContainer::new(),
                calendar_grid: CalendarGrid::new(),
                task_list: TaskList::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AgendaPopup {
        const NAME: &'static str = "AxisAgendaPopup";
        type Type = super::AgendaPopup;
        type ParentType = gtk4::ApplicationWindow;
    }

    impl ObjectImpl for AgendaPopup {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.init_layer_shell();
            obj.set_layer(Layer::Top);
            obj.set_namespace(Some("axis-agenda"));
            obj.set_anchor(Edge::Bottom, true);
            obj.set_anchor(Edge::Left, false);
            obj.set_anchor(Edge::Right, false);
            obj.set_margin(Edge::Bottom, 64);
            obj.set_keyboard_mode(KeyboardMode::OnDemand);
            obj.add_css_class("popup-window");

            let main_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
            main_box.add_css_class("qs-panel");
            main_box.set_margin_start(4);
            main_box.set_margin_end(4);
            main_box.set_margin_top(4);
            main_box.set_margin_bottom(4);

            main_box.append(&self.calendar_grid);
            
            let separator = gtk4::Separator::new(gtk4::Orientation::Vertical);
            main_box.append(&separator);
            
            main_box.append(&self.task_list);

            self.container.set_content(&main_box);
            obj.set_child(Some(&self.container));
        }
    }

    impl WidgetImpl for AgendaPopup {}
    impl WindowImpl for AgendaPopup {}
    impl ApplicationWindowImpl for AgendaPopup {}
}
