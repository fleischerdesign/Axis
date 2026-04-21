use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use crate::presentation::workspaces::WorkspaceView;
use axis_domain::models::workspaces::Workspace;
use std::cell::RefCell;
use std::rc::Rc;

glib::wrapper! {
    pub struct WorkspaceDots(ObjectSubclass<imp::WorkspaceDots>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl WorkspaceDots {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl WorkspaceView for WorkspaceDots {
    fn update_workspaces(&self, mut workspaces: Vec<Workspace>) {
        workspaces.sort_by_key(|w| w.id);
        let container = self.clone();
        let callback = self.imp().click_callback.borrow().clone();

        glib::idle_add_local(move || {
            let target = workspaces.len();

            while child_count(&container) > target {
                if let Some(last) = container.last_child() {
                    container.remove(&last);
                }
            }

            let mut child = container.first_child();
            for ws in workspaces.iter() {
                if let Some(existing_dot) = child {
                    if ws.is_active {
                        existing_dot.add_css_class("active");
                    } else {
                        existing_dot.remove_css_class("active");
                    }
                    child = existing_dot.next_sibling();
                } else {
                    // Neuen interaktiven Dot (Button) erstellen
                    let dot = gtk4::Button::builder()
                        .css_classes(["ws-dot"])
                        .valign(gtk4::Align::Center)
                        .build();
                    
                    if ws.is_active {
                        dot.add_css_class("active");
                    }

                    if let Some(cb) = &callback {
                        let ws_id = ws.id;
                        let cb_clone = cb.clone();
                        dot.connect_clicked(move |_| {
                            cb_clone(ws_id);
                        });
                    }

                    container.append(&dot);
                }
            }
            glib::ControlFlow::Break
        });
    }

    fn on_workspace_clicked(&self, f: Box<dyn Fn(u32) + Send + Sync>) {
        *self.imp().click_callback.borrow_mut() = Some(Rc::new(f));
    }
}

fn child_count(container: &WorkspaceDots) -> usize {
    let mut count = 0;
    let mut child = container.first_child();
    while child.is_some() {
        count += 1;
        child = child.and_then(|c| c.next_sibling());
    }
    count
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct WorkspaceDots {
        pub click_callback: RefCell<Option<Rc<Box<dyn Fn(u32) + Send + Sync>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WorkspaceDots {
        const NAME: &'static str = "WorkspaceDots";
        type Type = super::WorkspaceDots;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for WorkspaceDots {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_spacing(6);
            self.obj().add_css_class("workspace-dots");
        }
    }

    impl WidgetImpl for WorkspaceDots {}
    impl BoxImpl for WorkspaceDots {}
}
