use libadwaita::prelude::*;
use libadwaita as adw;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use axis_domain::models::agenda::AgendaStatus;
use axis_presentation::View;
use std::rc::Rc;
use std::cell::{Cell, RefCell};

glib::wrapper! {
    pub struct TaskList(ObjectSubclass<imp::TaskList>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl TaskList {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn on_list_changed(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.imp().list_changed_callback.borrow_mut() = Some(Rc::new(f));
    }

    pub fn render(&self, status: &AgendaStatus) {
        let imp = self.imp();
        
        // 1. Update Dropdown Model
        let current_count = imp.dropdown.model().map(|m| m.n_items()).unwrap_or(0);
        if current_count != status.task_lists.len() as u32 {
            let list_names: Vec<&str> = status.task_lists.iter().map(|l| l.title.as_str()).collect();
            let new_model = gtk4::StringList::new(&list_names);
            imp.dropdown.set_model(Some(&new_model));
        }

        // 2. Update Selection
        if let Some(selected_id) = &status.selected_list_id {
            if let Some(idx) = status.task_lists.iter().position(|l| l.id == *selected_id) {
                imp.is_updating_programmatically.set(true);
                imp.dropdown.set_selected(idx as u32);
                imp.is_updating_programmatically.set(false);
            }
        }

        *imp.current_task_lists.borrow_mut() = status.task_lists.clone();

        // 3. Loading State
        if status.is_loading_tasks {
            imp.spinner.start();
            imp.spinner.set_visible(true);
            imp.list_box.set_opacity(0.5);
        } else {
            imp.spinner.stop();
            imp.spinner.set_visible(false);
            imp.list_box.set_opacity(1.0);
        }

        // 4. Render Tasks
        while let Some(child) = imp.list_box.first_child() {
            imp.list_box.remove(&child);
        }

        if status.tasks.is_empty() && !status.is_loading_tasks {
            let empty = adw::StatusPage::builder()
                .title("Keine Aufgaben")
                .description("Alles erledigt!")
                .build();
            imp.list_box.append(&empty);
            return;
        }

        for task in &status.tasks {
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(12)
                .css_classes(["agenda-task-row"])
                .build();

            if task.done { row.add_css_class("done"); }

            let check = gtk4::CheckButton::builder()
                .active(task.done)
                .css_classes(["agenda-task-check"])
                .build();

            let label = gtk4::Label::builder()
                .label(&task.title)
                .hexpand(true)
                .halign(gtk4::Align::Start)
                .css_classes(["agenda-task-label"])
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();

            let delete_btn = gtk4::Button::builder()
                .icon_name("user-trash-symbolic")
                .css_classes(["flat", "agenda-task-delete"])
                .valign(gtk4::Align::Center)
                .build();

            row.append(&check);
            row.append(&label);
            row.append(&delete_btn);
            imp.list_box.append(&row);
        }
    }
}

impl View<AgendaStatus> for TaskList {
    fn render(&self, status: &AgendaStatus) {
        self.render(status);
    }
}

mod imp {
    use super::*;
    use axis_domain::models::tasks::TaskList as DomainTaskList;

    pub struct TaskList {
        pub list_box: gtk4::ListBox,
        pub header_box: gtk4::Box,
        pub scrolled: gtk4::ScrolledWindow,
        pub spinner: gtk4::Spinner,
        pub dropdown: gtk4::DropDown,
        pub is_updating_programmatically: Cell<bool>,
        pub current_task_lists: RefCell<Vec<DomainTaskList>>,
        pub list_changed_callback: RefCell<Option<Rc<Box<dyn Fn(String) + 'static>>>>,
    }

    impl Default for TaskList {
        fn default() -> Self {
            let dropdown = gtk4::DropDown::builder()
                .css_classes(["agenda-list-dropdown"])
                .valign(gtk4::Align::Center)
                .build();

            Self {
                list_box: gtk4::ListBox::builder()
                    .selection_mode(gtk4::SelectionMode::None)
                    .build(),
                header_box: gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(8)
                    .margin_bottom(8)
                    .build(),
                scrolled: gtk4::ScrolledWindow::builder()
                    .hscrollbar_policy(gtk4::PolicyType::Never)
                    .vscrollbar_policy(gtk4::PolicyType::Automatic)
                    .min_content_height(400)
                    .build(),
                spinner: gtk4::Spinner::builder()
                    .valign(gtk4::Align::Center)
                    .margin_start(8)
                    .build(),
                dropdown,
                is_updating_programmatically: Cell::new(false),
                current_task_lists: RefCell::new(Vec::new()),
                list_changed_callback: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TaskList {
        const NAME: &'static str = "AxisTaskList";
        type Type = super::TaskList;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for TaskList {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_orientation(gtk4::Orientation::Vertical);
            obj.set_spacing(12);
            obj.set_width_request(280);

            let label = gtk4::Label::builder()
                .label("Aufgaben")
                .css_classes(["agenda-section-header"])
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build();

            self.header_box.append(&label);
            self.header_box.append(&self.dropdown);
            self.header_box.append(&self.spinner);
            obj.append(&self.header_box);

            self.list_box.add_css_class("background-none");
            self.scrolled.set_child(Some(&self.list_box));
            obj.append(&self.scrolled);

            let obj_c = obj.clone();
            self.dropdown.connect_selected_notify(move |dd| {
                let imp = obj_c.imp();
                if imp.is_updating_programmatically.get() {
                    return;
                }
                
                let selected = dd.selected();
                let lists = imp.current_task_lists.borrow();
                if let Some(list) = lists.get(selected as usize) {
                    if let Some(cb) = imp.list_changed_callback.borrow().as_ref() {
                        cb(list.id.clone());
                    }
                }
            });
        }
    }

    impl WidgetImpl for TaskList {}
    impl BoxImpl for TaskList {}
}
