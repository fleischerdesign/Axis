pub struct ScrolledList {
    pub scrolled: gtk4::ScrolledWindow,
    pub list: gtk4::ListBox,
}

const DEFAULT_MIN_CONTENT_HEIGHT: i32 = 300;

impl ScrolledList {
    pub fn new(min_content_height: i32) -> Self {
        Self::with_height(min_content_height)
    }

    pub fn with_default_height() -> Self {
        Self::with_height(DEFAULT_MIN_CONTENT_HEIGHT)
    }

    pub fn with_height(min_content_height: i32) -> Self {
        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .min_content_height(min_content_height)
            .build();
        scrolled.set_child(Some(&list));

        Self { scrolled, list }
    }
}
