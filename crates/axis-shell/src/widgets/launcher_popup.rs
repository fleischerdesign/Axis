use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::{glib, gio};
use axis_domain::models::launcher::LauncherItem;
use axis_domain::models::popups::PopupType;
use crate::widgets::popup_base::PopupContainer;
use crate::widgets::components::list_row::ListRow;
use crate::presentation::popups::PopupView;
use crate::presentation::launcher::LauncherView;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;
use std::rc::Rc;

glib::wrapper! {
    pub struct LauncherPopup(ObjectSubclass<imp::LauncherPopup>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

pub struct RowEntry {
    row: ListRow,
    list_box_row: gtk4::ListBoxRow,
}

pub struct Callbacks {
    on_search: RefCell<Option<Box<dyn Fn(&str) + 'static>>>,
    on_select_next: RefCell<Option<Box<dyn Fn() + 'static>>>,
    on_select_prev: RefCell<Option<Box<dyn Fn() + 'static>>>,
    on_activate: RefCell<Option<Box<dyn Fn(Option<usize>) + 'static>>>,
    on_escape: RefCell<Option<Box<dyn Fn() + 'static>>>,
}

impl Callbacks {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            on_search: RefCell::new(None),
            on_select_next: RefCell::new(None),
            on_select_prev: RefCell::new(None),
            on_activate: RefCell::new(None),
            on_escape: RefCell::new(None),
        })
    }
}

impl LauncherPopup {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }

    pub fn on_search(&self, f: Box<dyn Fn(&str) + 'static>) {
        *self.imp().callbacks.on_search.borrow_mut() = Some(f);
    }

    pub fn on_select_next(&self, f: Box<dyn Fn() + 'static>) {
        *self.imp().callbacks.on_select_next.borrow_mut() = Some(f);
    }

    pub fn on_select_prev(&self, f: Box<dyn Fn() + 'static>) {
        *self.imp().callbacks.on_select_prev.borrow_mut() = Some(f);
    }

    pub fn on_activate(&self, f: Box<dyn Fn(Option<usize>) + 'static>) {
        *self.imp().callbacks.on_activate.borrow_mut() = Some(f);
    }

    pub fn on_escape(&self, f: Box<dyn Fn() + 'static>) {
        *self.imp().callbacks.on_escape.borrow_mut() = Some(f);
    }

    pub fn update_results(&self, results: &[LauncherItem], selected_index: Option<usize>) {
        let imp = self.imp();
        let mut rows = imp.rows.borrow_mut();
        let list = &imp.list;
        let scrolled = &imp.scrolled;
        let d_title = &imp.detail_title;
        let d_desc = &imp.detail_desc;
        let d_rev = &imp.detail_revealer;

        let new_ids: std::collections::HashSet<&str> = results.iter().map(|r| r.id.as_str()).collect();

        let stale: Vec<String> = rows
            .keys()
            .filter(|id| !new_ids.contains(id.as_str()))
            .cloned()
            .collect();
        for id in stale {
            if let Some(entry) = rows.remove(&id) {
                list.remove(&entry.list_box_row);
            }
        }

        for (idx, item) in results.iter().enumerate() {
            if let Some(entry) = rows.get(&item.id) {
                entry.row.set_title(&item.title);
                entry.row.set_icon(&item.icon_name);
                entry.row.set_subtitle(item.description.as_deref());
                continue;
            }

            let row = ListRow::new(&item.title, &item.icon_name);
            row.set_subtitle(item.description.as_deref());
            let list_box_row = gtk4::ListBoxRow::builder()
                .selectable(false)
                .activatable(true)
                .child(&row)
                .build();

            rows.insert(
                item.id.clone(),
                RowEntry { row, list_box_row: list_box_row.clone() },
            );
            list.insert(&list_box_row, idx as i32);
        }

        if results.is_empty() {
            list.unselect_all();
            d_rev.set_reveal_child(false);
            imp.container.set_width_request(380);
            for entry in rows.values() {
                entry.row.remove_css_class("launcher-selected");
            }
        } else if let Some(idx) = selected_index {
            for entry in rows.values() {
                entry.row.remove_css_class("launcher-selected");
            }

            if let Some(row) = list.row_at_index(idx as i32) {
                list.select_row(Some(&row));

                for entry in rows.values() {
                    if entry.list_box_row.index() == row.index() {
                        entry.row.add_css_class("launcher-selected");
                        break;
                    }
                }

                let adj = scrolled.vadjustment();
                if let Some(point) = row.compute_point(&*list, &gtk4::graphene::Point::new(0.0, 0.0)) {
                    let row_y = point.y() as f64;
                    let row_h = row.height() as f64;
                    let page = adj.page_size();
                    let target = (row_y - (page / 2.0) + (row_h / 2.0)).clamp(0.0, adj.upper() - page);
                    smooth_scroll(&adj, target);
                }

                if let Some(item) = results.get(idx) {
                    d_title.set_text(&item.title);
                    d_desc.set_text(item.description.as_deref().unwrap_or(""));
                    d_rev.set_reveal_child(true);
                    imp.container.set_width_request(680);
                }
            }
        }
    }

    pub fn clear_and_focus(&self) {
        let imp = self.imp();
        imp.entry.set_text("");
        imp.entry.grab_focus();

        if let Some(f) = imp.callbacks.on_search.borrow().as_ref() {
            f("");
        }
    }
}

fn smooth_scroll(adj: &gtk4::Adjustment, target: f64) {
    const STEPS: u32 = 8;
    const INTERVAL: std::time::Duration = std::time::Duration::from_millis(15);

    let start = adj.value();
    let delta = target - start;
    if delta.abs() < 1.0 { return; }

    let adj = adj.clone();
    let step = Rc::new(Cell::new(0u32));

    glib::timeout_add_local(INTERVAL, move || {
        let s = step.get() + 1;
        step.set(s);

        let t = s as f64 / STEPS as f64;
        let ease = 1.0 - (1.0 - t).powi(3);
        adj.set_value(start + delta * ease);

        if s >= STEPS {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

impl PopupView for LauncherPopup {
    fn get_type(&self) -> PopupType { PopupType::Launcher }
    fn popup_container(&self) -> PopupContainer { self.imp().container.clone() }
    fn popup_window(&self) -> gtk4::ApplicationWindow { self.clone().upcast() }

    fn show(&self) {
        let win = self.clone();
        glib::idle_add_local(move || {
            win.popup_container().animate_show(&win.popup_window());
            win.clear_and_focus();
            glib::ControlFlow::Break
        });
    }
}

impl LauncherView for LauncherPopup {
    fn render_results(&self, results: &[LauncherItem], selected_index: Option<usize>) {
        self.update_results(results, selected_index);
    }
    fn clear_and_focus(&self) {
        self.clear_and_focus();
    }
}

mod imp {
    use super::*;
    use gtk4_layer_shell::{LayerShell, Layer, Edge, KeyboardMode};

    pub struct LauncherPopup {
        pub container: PopupContainer,
        pub entry: gtk4::Entry,
        pub list: gtk4::ListBox,
        pub scrolled: gtk4::ScrolledWindow,
        pub detail_title: gtk4::Label,
        pub detail_desc: gtk4::Label,
        pub detail_revealer: gtk4::Revealer,
        pub rows: RefCell<HashMap<String, RowEntry>>,
        pub callbacks: Arc<Callbacks>,
    }

    impl Default for LauncherPopup {
        fn default() -> Self {
            let container = PopupContainer::new();

            let entry = gtk4::Entry::builder()
                .placeholder_text("Suchen, Finden, Machen...")
                .hexpand(true)
                .css_classes(vec!["qs-entry"])
                .build();

            let list = gtk4::ListBox::builder()
                .css_classes(vec!["qs-list", "launcher-list"])
                .selection_mode(gtk4::SelectionMode::Single)
                .build();

            let scrolled = gtk4::ScrolledWindow::builder()
                .hscrollbar_policy(gtk4::PolicyType::Never)
                .vscrollbar_policy(gtk4::PolicyType::Automatic)
                .vexpand(true)
                .min_content_height(400)
                .max_content_height(400)
                .build();
            scrolled.add_css_class("qs-scrolled");

            let detail_revealer = gtk4::Revealer::builder()
                .transition_type(gtk4::RevealerTransitionType::SlideRight)
                .transition_duration(250)
                .build();

            let detail_title = gtk4::Label::builder()
                .halign(gtk4::Align::Start)
                .css_classes(vec!["subpage-title"])
                .wrap(true)
                .build();

            let detail_desc = gtk4::Label::builder()
                .halign(gtk4::Align::Start)
                .wrap(true)
                .css_classes(vec!["list-sublabel"])
                .build();

            Self {
                container,
                entry,
                list,
                scrolled,
                detail_title,
                detail_desc,
                detail_revealer,
                rows: RefCell::new(HashMap::new()),
                callbacks: Callbacks::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LauncherPopup {
        const NAME: &'static str = "AxisLauncherPopup";
        type Type = super::LauncherPopup;
        type ParentType = gtk4::ApplicationWindow;
    }

    impl ObjectImpl for LauncherPopup {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.init_layer_shell();
            obj.set_layer(Layer::Top);
            obj.set_namespace(Some("axis-launcher"));
            obj.set_anchor(Edge::Left, true);
            obj.set_anchor(Edge::Bottom, true);
            obj.set_margin(Edge::Bottom, 64);
            obj.set_margin(Edge::Left, 10);
            obj.set_default_size(380, -1);
            obj.set_keyboard_mode(KeyboardMode::Exclusive);
            obj.add_css_class("popup-window");

            let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            container.add_css_class("qs-panel");
            container.set_width_request(380);

            let left_pane = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            left_pane.set_width_request(380);
            left_pane.set_hexpand(true);

            let entry_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            entry_box.set_margin_bottom(12);
            entry_box.append(&self.entry);
            left_pane.append(&entry_box);

            self.scrolled.set_child(Some(&self.list));
            left_pane.append(&self.scrolled);

            container.append(&left_pane);

            let detail_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
            detail_box.set_width_request(280);
            detail_box.set_margin_start(16);
            detail_box.set_margin_end(16);
            detail_box.add_css_class("launcher-details");
            detail_box.append(&self.detail_title);
            detail_box.append(&self.detail_desc);
            self.detail_revealer.set_child(Some(&detail_box));
            container.append(&self.detail_revealer);

            self.container.set_content(&container);
            obj.set_child(Some(&self.container));

            let cbs = self.callbacks.clone();
            let list = self.list.clone();
            list.connect_row_activated(move |_, row| {
                let idx = row.index() as usize;
                if let Some(f) = cbs.on_activate.borrow().as_ref() {
                    f(Some(idx));
                }
            });

            let cbs = self.callbacks.clone();
            let entry = self.entry.clone();
            entry.connect_changed(move |e| {
                if let Some(f) = cbs.on_search.borrow().as_ref() {
                    f(&e.text());
                }
            });

            let cbs = self.callbacks.clone();
            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, key, _, state| {
                use gtk4::gdk::{Key, ModifierType};
                match key {
                    Key::Escape => {
                        if let Some(f) = cbs.on_escape.borrow().as_ref() { f(); }
                        gtk4::glib::Propagation::Stop
                    }
                    Key::Down => {
                        if let Some(f) = cbs.on_select_next.borrow().as_ref() { f(); }
                        gtk4::glib::Propagation::Stop
                    }
                    Key::Up => {
                        if let Some(f) = cbs.on_select_prev.borrow().as_ref() { f(); }
                        gtk4::glib::Propagation::Stop
                    }
                    Key::Tab => {
                        if state.contains(ModifierType::SHIFT_MASK) {
                            if let Some(f) = cbs.on_select_prev.borrow().as_ref() { f(); }
                        } else {
                            if let Some(f) = cbs.on_select_next.borrow().as_ref() { f(); }
                        }
                        gtk4::glib::Propagation::Stop
                    }
                    _ => gtk4::glib::Propagation::Proceed,
                }
            });
            self.entry.add_controller(key_controller);

            let cbs = self.callbacks.clone();
            self.entry.connect_activate(move |_| {
                if let Some(f) = cbs.on_activate.borrow().as_ref() {
                    f(None);
                }
            });
        }
    }

    impl WidgetImpl for LauncherPopup {}
    impl WindowImpl for LauncherPopup {}
    impl ApplicationWindowImpl for LauncherPopup {}
}
