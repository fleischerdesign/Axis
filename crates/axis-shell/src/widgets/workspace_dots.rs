use libadwaita::prelude::*;
use axis_domain::models::workspaces::WorkspaceStatus;
use axis_presentation::View;
use std::cell::Cell;

#[derive(Clone)]
pub struct WorkspaceDots {
    pub container: gtk4::Box,
    dot_count: Cell<usize>,
}

impl WorkspaceDots {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        container.add_css_class("workspace-dots");
        Self {
            container,
            dot_count: Cell::new(0),
        }
    }
}

impl View<WorkspaceStatus> for WorkspaceDots {
    fn render(&self, status: &WorkspaceStatus) {
        let mut workspaces = status.workspaces.clone();
        workspaces.sort_by_key(|w| w.id);

        let target = workspaces.len();

        while self.dot_count.get() > target {
            if let Some(last) = self.container.last_child() {
                self.container.remove(&last);
                self.dot_count.set(self.dot_count.get() - 1);
            }
        }

        let mut child = self.container.first_child();
        for ws in workspaces.iter() {
            if let Some(existing_dot) = child {
                if ws.is_active {
                    existing_dot.add_css_class("active");
                } else {
                    existing_dot.remove_css_class("active");
                }
                child = existing_dot.next_sibling();
            } else {
                let dot = gtk4::Box::builder()
                    .css_classes(["ws-dot"])
                    .valign(gtk4::Align::Center)
                    .build();

                if ws.is_active {
                    dot.add_css_class("active");
                }

                self.container.append(&dot);
                self.dot_count.set(self.dot_count.get() + 1);
            }
        }
    }
}
