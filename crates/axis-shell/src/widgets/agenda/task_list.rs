use libadwaita::prelude::*;
use libadwaita as adw;
use gtk4::glib;
use axis_domain::models::agenda::AgendaStatus;
use axis_presentation::View;
use std::rc::Rc;
use std::cell::{Cell, RefCell};

#[derive(Clone)]
pub struct TaskList {
    pub container: gtk4::Box,
    list_box: gtk4::ListBox,
    scrolled: gtk4::ScrolledWindow,
    spinner: gtk4::Spinner,
    dropdown: gtk4::DropDown,
    entry: gtk4::Entry,
    add_button: gtk4::Button,
    is_updating_programmatically: Rc<Cell<bool>>,
    current_task_lists: Rc<RefCell<Vec<axis_domain::models::tasks::TaskList>>>,
    list_changed_callback: Rc<RefCell<Option<Rc<Box<dyn Fn(String) + 'static>>>>>,
    task_toggled_callback: Rc<RefCell<Option<Rc<Box<dyn Fn(String, bool) + 'static>>>>>,
    task_deleted_callback: Rc<RefCell<Option<Rc<Box<dyn Fn(String) + 'static>>>>>,
    task_created_callback: Rc<RefCell<Option<Rc<Box<dyn Fn(String) + 'static>>>>>,
}

impl TaskList {
    pub fn new() -> Self {
        let dropdown = gtk4::DropDown::builder()
            .css_classes(["agenda-list-dropdown"])
            .valign(gtk4::Align::Center)
            .build();

        let entry = gtk4::Entry::builder()
            .placeholder_text("New task...")
            .css_classes(["agenda-task-entry"])
            .hexpand(true)
            .build();

        let add_button = gtk4::Button::builder()
            .icon_name("list-add-symbolic")
            .css_classes(["flat", "agenda-task-add-btn"])
            .valign(gtk4::Align::Center)
            .build();

        let list_box = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        list_box.add_css_class("background-none");

        let spinner = gtk4::Spinner::builder()
            .valign(gtk4::Align::Center)
            .margin_start(8)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_height(350)
            .build();
        scrolled.set_child(Some(&list_box));

        let header_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_bottom(8)
            .build();

        let label = gtk4::Label::builder()
            .label("Tasks")
            .css_classes(["agenda-section-header"])
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();
        header_box.append(&label);
        header_box.append(&dropdown);
        header_box.append(&spinner);

        let input_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .build();
        input_row.append(&entry);
        input_row.append(&add_button);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        container.set_width_request(280);
        container.append(&header_box);
        container.append(&scrolled);
        container.append(&input_row);

        let tl = Self {
            container,
            list_box,
            scrolled,
            spinner,
            dropdown,
            entry,
            add_button,
            is_updating_programmatically: Rc::new(Cell::new(false)),
            current_task_lists: Rc::new(RefCell::new(Vec::new())),
            list_changed_callback: Rc::new(RefCell::new(None)),
            task_toggled_callback: Rc::new(RefCell::new(None)),
            task_deleted_callback: Rc::new(RefCell::new(None)),
            task_created_callback: Rc::new(RefCell::new(None)),
        };

        let tl_c = tl.clone();
        let entry_c = tl.entry.clone();
        let on_add = move || {
            let title = entry_c.text().to_string();
            if title.is_empty() { return; }
            if let Some(cb) = tl_c.task_created_callback.borrow().as_ref() {
                cb(title);
                entry_c.set_text("");
            }
        };

        let on_add_btn = on_add.clone();
        tl.add_button.connect_clicked(move |_| {
            on_add_btn();
        });

        tl.entry.connect_activate(move |_| {
            on_add();
        });

        let tl_c = tl.clone();
        tl.dropdown.connect_selected_notify(move |dd| {
            if tl_c.is_updating_programmatically.get() {
                return;
            }
            let selected = dd.selected();
            let lists = tl_c.current_task_lists.borrow();
            if let Some(list) = lists.get(selected as usize) {
                if let Some(cb) = tl_c.list_changed_callback.borrow().as_ref() {
                    cb(list.id.clone());
                }
            }
        });

        tl
    }

    pub fn on_list_changed(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.list_changed_callback.borrow_mut() = Some(Rc::new(f));
    }

    pub fn on_task_toggled(&self, f: Box<dyn Fn(String, bool) + 'static>) {
        *self.task_toggled_callback.borrow_mut() = Some(Rc::new(f));
    }

    pub fn on_task_deleted(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.task_deleted_callback.borrow_mut() = Some(Rc::new(f));
    }

    pub fn on_task_created(&self, f: Box<dyn Fn(String) + 'static>) {
        *self.task_created_callback.borrow_mut() = Some(Rc::new(f));
    }

    pub fn render(&self, status: &AgendaStatus) {
        let current_count = self.dropdown.model().map(|m| m.n_items()).unwrap_or(0);
        if current_count != status.task_lists.len() as u32 {
            let list_names: Vec<&str> = status.task_lists.iter().map(|l| l.title.as_str()).collect();
            let new_model = gtk4::StringList::new(&list_names);
            self.dropdown.set_model(Some(&new_model));
        }

        if let Some(selected_id) = &status.selected_list_id {
            if let Some(idx) = status.task_lists.iter().position(|l| l.id == *selected_id) {
                self.is_updating_programmatically.set(true);
                self.dropdown.set_selected(idx as u32);
                self.is_updating_programmatically.set(false);
            }
        }

        *self.current_task_lists.borrow_mut() = status.task_lists.clone();

        if status.is_loading_tasks {
            self.spinner.start();
            self.spinner.set_visible(true);
            self.list_box.set_opacity(0.5);
        } else {
            self.spinner.stop();
            self.spinner.set_visible(false);
            self.list_box.set_opacity(1.0);
        }

        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        if status.tasks.is_empty() && !status.is_loading_tasks {
            let empty = adw::StatusPage::builder()
                .title("No tasks")
                .description("All done!")
                .build();
            self.list_box.append(&empty);
        } else {
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

                let task_id = task.id.clone();
                let callback = self.task_toggled_callback.borrow().clone();
                check.connect_toggled(move |btn| {
                    if let Some(cb) = &callback {
                        cb(task_id.clone(), btn.is_active());
                    }
                });

                let label = gtk4::Label::builder()
                    .label(&task.title)
                    .hexpand(true)
                    .halign(gtk4::Align::Start)
                    .css_classes(["agenda-task-label"])
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .build();

                let task_id_del = task.id.clone();
                let callback_del = self.task_deleted_callback.borrow().clone();
                let delete_btn = gtk4::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .css_classes(["flat", "agenda-task-delete"])
                    .valign(gtk4::Align::Center)
                    .build();

                delete_btn.connect_clicked(move |_| {
                    if let Some(cb) = &callback_del {
                        cb(task_id_del.clone());
                    }
                });

                row.append(&check);
                row.append(&label);
                row.append(&delete_btn);
                self.list_box.append(&row);
            }
        }
    }
}

impl View<AgendaStatus> for TaskList {
    fn render(&self, status: &AgendaStatus) {
        self.render(status);
    }
}
