use libadwaita::prelude::*;
use libadwaita as adw;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use axis_domain::models::agenda::AgendaStatus;
use axis_presentation::View;
use std::rc::Rc;
use std::cell::Cell;

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
        
        // 1. Update Dropdown (Header) - ONLY IF NOT BUILT OR LISTS CHANGED
        let needs_rebuild = !imp.dropdown_built.get() || 
                           imp.last_list_count.get() != status.task_lists.len();

        if needs_rebuild && !status.task_lists.is_empty() {
            while let Some(child) = imp.header_box.first_child() {
                imp.header_box.remove(&child);
            }

            let label = gtk4::Label::builder()
                .label("Aufgaben")
                .css_classes(["agenda-section-header"])
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build();
            imp.header_box.append(&label);

            let list_names: Vec<&str> = status.task_lists.iter().map(|l| l.title.as_str()).collect();
            let model = gtk4::StringList::new(&list_names);
            let dropdown = gtk4::DropDown::builder()
                .model(&model)
                .css_classes(["agenda-list-dropdown"])
                .valign(gtk4::Align::Center)
                .build();

            if let Some(selected_id) = &status.selected_list_id {
                if let Some(idx) = status.task_lists.iter().position(|l| l.id == *selected_id) {
                    dropdown.set_selected(idx as u32);
                }
            }

            let task_lists = status.task_lists.clone();
            let callback = imp.list_changed_callback.borrow().clone();
            let selected_list_id = status.selected_list_id.clone();
            
            dropdown.connect_selected_notify(move |dd| {
                if let Some(cb) = &callback {
                    let selected = dd.selected();
                    if let Some(list) = task_lists.get(selected as usize) {
                        // ONLY notify if the selection differs from the last known state
                        if Some(list.id.clone()) != selected_list_id {
                            cb(list.id.clone());
                        }
                    }
                }
            });

            imp.header_box.append(&dropdown);
            imp.dropdown_built.set(true);
            imp.last_list_count.set(status.task_lists.len());
        } else if let Some(dropdown) = imp.header_box.last_child().and_downcast::<gtk4::DropDown>() {
            // Dropdown exists, just sync selection if it differs from status
            if let Some(selected_id) = &status.selected_list_id {
                if let Some(idx) = status.task_lists.iter().position(|l| l.id == *selected_id) {
                    if dropdown.selected() != idx as u32 {
                        dropdown.set_selected(idx as u32);
                    }
                }
            }
        }

        // 2. Clear and Render Tasks
        while let Some(child) = imp.list_box.first_child() {
            imp.list_box.remove(&child);
        }

        if status.tasks.is_empty() {
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

            if task.done {
                row.add_css_class("done");
            }

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
    use std::cell::RefCell;

    pub struct TaskList {
        pub list_box: gtk4::ListBox,
        pub header_box: gtk4::Box,
        pub scrolled: gtk4::ScrolledWindow,
        pub dropdown_built: Cell<bool>,
        pub last_list_count: Cell<usize>,
        pub list_changed_callback: RefCell<Option<Rc<Box<dyn Fn(String) + 'static>>>>,
    }

    impl Default for TaskList {
        fn default() -> Self {
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
                dropdown_built: Cell::new(false),
                last_list_count: Cell::new(0),
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

            obj.append(&self.header_box);

            self.list_box.set_selection_mode(gtk4::SelectionMode::None);
            self.list_box.add_css_class("background-none"); // Custom class to ensure no bg
            self.scrolled.set_child(Some(&self.list_box));
            obj.append(&self.scrolled);
        }
    }

    impl WidgetImpl for TaskList {}
    impl BoxImpl for TaskList {}
}
