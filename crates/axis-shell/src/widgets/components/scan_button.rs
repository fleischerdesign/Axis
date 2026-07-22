use gtk4::prelude::*;

pub struct ScanButton {
    button: gtk4::Button,
    spinner: gtk4::Spinner,
    stack: gtk4::Stack,
}

impl ScanButton {
    pub fn new() -> Self {
        let image = gtk4::Image::from_icon_name("view-refresh-symbolic");
        let spinner = gtk4::Spinner::new();

        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();

        stack.add_named(&image, Some("icon"));
        stack.add_named(&spinner, Some("spinner"));
        stack.set_visible_child_name("icon");

        let button = gtk4::Button::builder()
            .child(&stack)
            .tooltip_text("Scan for networks")
            .css_classes(vec!["circular".to_string(), "flat".to_string()])
            .valign(gtk4::Align::Center)
            .build();

        Self {
            button,
            spinner,
            stack,
        }
    }

    pub fn widget(&self) -> &gtk4::Button {
        &self.button
    }

    pub fn set_scanning(&self, is_scanning: bool) {
        if is_scanning {
            self.spinner.start();
            self.stack.set_visible_child_name("spinner");
            self.button.set_sensitive(false);
        } else {
            self.spinner.stop();
            self.stack.set_visible_child_name("icon");
            self.button.set_sensitive(true);
        }
    }

    pub fn connect_clicked<F: Fn() + 'static>(&self, f: F) {
        let btn = self.button.clone();
        let spin = self.spinner.clone();
        let st = self.stack.clone();
        self.button.connect_clicked(move |_| {
            spin.start();
            st.set_visible_child_name("spinner");
            btn.set_sensitive(false);
            f();
        });
    }
}

impl Default for ScanButton {
    fn default() -> Self {
        Self::new()
    }
}
