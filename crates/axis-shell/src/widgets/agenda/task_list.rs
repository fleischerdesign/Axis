use libadwaita::prelude::*;
use libadwaita as adw;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use axis_domain::models::agenda::AgendaStatus;
use axis_presentation::View;

glib::wrapper! {
    pub struct TaskList(ObjectSubclass<imp::TaskList>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl TaskList {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn render(&self, status: &AgendaStatus) {
        let imp = self.imp();
        
        // Clear existing
        while let Some(child) = imp.list_box.first_child() {
            imp.list_box.remove(&child);
        }

        if status.events.is_empty() && status.tasks.is_empty() {
            let empty = adw::StatusPage::builder()
                .title("Nothing scheduled")
                .description("Enjoy your free time.")
                .build();
            imp.list_box.append(&empty);
            return;
        }

        // Render Events
        if !status.events.is_empty() {
            let header = gtk4::Label::builder()
                .label("Events")
                .halign(gtk4::Align::Start)
                .css_classes(["agenda-section-header"])
                .build();
            imp.list_box.append(&header);

            for event in &status.events {
                let row = adw::ActionRow::builder()
                    .title(&event.summary)
                    .subtitle(&format!("{} - {}", event.start, event.end))
                    .build();
                imp.list_box.append(&row);
            }
        }

        // Render Tasks
        if !status.tasks.is_empty() {
            let header = gtk4::Label::builder()
                .label("Tasks")
                .halign(gtk4::Align::Start)
                .css_classes(["agenda-section-header"])
                .margin_top(12)
                .build();
            imp.list_box.append(&header);

            for task in &status.tasks {
                let check = gtk4::CheckButton::new();
                check.set_active(task.done);
                
                let row = adw::ActionRow::builder()
                    .title(&task.title)
                    .build();
                row.add_prefix(&check);
                imp.list_box.append(&row);
            }
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

    pub struct TaskList {
        pub list_box: gtk4::ListBox,
        pub scrolled: gtk4::ScrolledWindow,
    }

    impl Default for TaskList {
        fn default() -> Self {
            Self {
                list_box: gtk4::ListBox::builder()
                    .selection_mode(gtk4::SelectionMode::None)
                    .css_classes(["boxed-list"])
                    .build(),
                scrolled: gtk4::ScrolledWindow::builder()
                    .hscrollbar_policy(gtk4::PolicyType::Never)
                    .vscrollbar_policy(gtk4::PolicyType::Automatic)
                    .min_content_height(350)
                    .build(),
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
            obj.set_width_request(320);

            self.scrolled.set_child(Some(&self.list_box));
            obj.append(&self.scrolled);
        }
    }

    impl WidgetImpl for TaskList {}
    impl BoxImpl for TaskList {}
}
